/* Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software
 * and associated documentation files (the "Software"), to deal in the Software without restriction,
 * including without limitation the rights to use, copy, modify, merge, publish, distribute,
 * sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or
 * substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
 * NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! This module defines the tokenizer used by the HTML parser.

use {
    alloc::{
        // TODO: string::String,
        vec::Vec
    },
    core::{
        cell::RefCell,
        marker::PhantomData,
        mem
    },
    crate::{
        interned_string::InternedString,
        namespace
    },
    crate::shim::String,
    super::{
        Parser,
        ParseError,
        element::Attribute,
        encoding::CharEncoding
    }
};

#[derive(Debug)]
pub(crate) struct Tokenizer<A: alloc::alloc::Allocator+Copy> {
    allocator: A,
    pub(super) encoding: RefCell<CharEncoding>,
    pub(super) state: TokenizerState,
    return_state: Option<TokenizerState>,
    reconsuming_character: Option<char>,
    temporary_buffer: String<A>,
    current_token: Option<Token<A>>,
    discarding_attribute: bool,
    file_ended: bool,
    _phantom: PhantomData<A>
}

impl<A: alloc::alloc::Allocator+Copy> Tokenizer<A> {
    pub(crate) fn new(allocator: A) -> Self {
        Self {
            allocator,
            encoding: RefCell::new(CharEncoding::Utf8),
            state: TokenizerState::Data,
            return_state: None,
            reconsuming_character: None,
            temporary_buffer: String::new_in(allocator),
            current_token: None,
            discarding_attribute: false,
            file_ended: false,
            _phantom: PhantomData
        }
    }

    /// Returns the next token, in the style of an iterator.
    pub(super) fn tokenize<I: Iterator<Item = char>>(
            &mut self,
            input_characters: &mut I,
            parser: &Parser<A>,
            eof: bool
    ) -> Option<Result<Token<A>, (Token<A>, ParseError)>> {
        if self.file_ended {
            return None;
        }

        loop {
            let current_input_character = match self.reconsuming_character {
                Some(c) => {
                    self.reconsuming_character = None;
                    Some(c)
                },
                None => {
                    let mut next_input_character = input_characters.next();
                    match next_input_character {
                        Some('\r') => {
                            // Per "Preprocessing the input stream", newlines must be normalized by changing
                            // "\r\n" into "\n", then changing any remaining "\r" into "\n".
                            next_input_character = Some('\n');
                            let next_next_input_character = input_characters.next();
                            match next_next_input_character {
                                Some('\n') => {},
                                _ => {
                                    // It wasn't CRLF, so keep the next character around.
                                    self.reconsuming_character = next_next_input_character;
                                }
                            };
                        },
                        Some(c) if c.is_control() && !c.is_ascii_whitespace() && c != '\0' => {
                            // TODO: Parse error: "control-character-in-input-stream"
                        },
                        _ => {}
                    };
                    next_input_character
                }
            };

            if current_input_character.is_none() && !eof {
                // Found the end of the stream, but not of the file. Just return.
                return None;
            }

            if let Some(token_result) = self.parse_character(current_input_character, parser) {
                return Some(token_result);
            }

            if current_input_character.is_none() {
                // Found the end of the file.
                self.file_ended = true;
                return Some(Ok(Token::EndOfFile));
            }
        }
    }

    fn parse_character(
            &mut self,
            current_input_character: Option<char>,
            parser: &Parser<A>
    ) -> Option<Result<Token<A>, (Token<A>, ParseError)>> {
        match self.state {
            TokenizerState::Data => match current_input_character {
                Some('&') => {
                    assert!(self.return_state.is_none());
                    self.return_state = Some(TokenizerState::Data);
                    self.state = TokenizerState::CharacterReference;
                },
                Some('<') => {
                    self.state = TokenizerState::TagOpen;
                },
                Some('\0') => {
                    return Some(Err(
                        (Token::Character('\0'), ParseError::UnexpectedNullCharacter)
                    ));
                },
                Some(c) => {
                    return Some(Ok(Token::Character(c)));
                },
                None => {
                    return None;
                }
            },
            TokenizerState::Rcdata => todo!(),
            TokenizerState::Rawtext => todo!(),
            TokenizerState::ScriptData => todo!(),
            TokenizerState::Plaintext => match current_input_character {
                Some('\0') => {
                    return Some(Err((Token::Character('\u{fffd}'), ParseError::UnexpectedNullCharacter)));
                },
                Some(c) => {
                    return Some(Ok(Token::Character(c)));
                },
                None => {
                    return None;
                }
            },
            TokenizerState::TagOpen => match current_input_character {
                Some('!') => {
                    assert!(self.temporary_buffer.is_empty());
                    self.state = TokenizerState::MarkupDeclarationOpen;
                },
                Some('/') => {
                    self.state = TokenizerState::EndTagOpen;
                },
                Some(c) if c.is_ascii_alphabetic() => {
                    self.state = TokenizerState::TagName;
                    assert!(self.current_token.is_none(), "unexpected token");
                    self.current_token = Some(Token::StartTag(Tag::new(self.allocator)));
                    return self.parse_character(current_input_character, parser);
                },
                Some('?') => {
                    // TODO: Parse error: "unexpected-question-mark-instead-of-tag-name"
                    self.state = TokenizerState::BogusComment;
                    return self.parse_character(current_input_character, parser);
                },
                Some(_) => {
                    // TODO: Parse error: "invalid-first-character-of-tag-name"
                    assert!(self.reconsuming_character.is_none());
                    self.reconsuming_character = current_input_character;
                    return Some(Ok(Token::Character('<')));
                },
                None => {
                    return Some(Err(
                        (Token::Character('<'), ParseError::EofBeforeTagName)
                    ));
                }
            },
            TokenizerState::EndTagOpen => match current_input_character {
                Some(c) if c.is_ascii_alphabetic() => {
                    assert!(self.current_token.is_none(), "unexpected token");
                    self.current_token = Some(Token::EndTag(Tag::new(self.allocator)));
                    self.state = TokenizerState::TagName;
                    return self.parse_character(current_input_character, parser);
                },
                Some('>') => {
                    // TODO: Parse error: "missing-end-tag-name"
                    self.state = TokenizerState::Data;
                },
                Some(c) => {
                    // TODO: Parse error: "invalid-first-character-of-tag-name"
                    assert!(self.current_token.is_none(), "unexpected token");
                    self.current_token = Some(Token::Comment(String::new_in(self.allocator)));
                    self.state = TokenizerState::BogusComment;
                    return self.parse_character(current_input_character, parser);
                },
                None => {
                    // TODO: Parse error: "eof-before-tag-name"
                    self.state = TokenizerState::EmitCharacters("</");
                    return self.parse_character(current_input_character, parser);
                }
            },
            TokenizerState::TagName => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    self.state = TokenizerState::BeforeAttributeName;
                },
                Some('/') => {
                    self.state = TokenizerState::SelfClosingStartTag;
                },
                Some('>') => {
                    self.state = TokenizerState::Data;
                    let token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::StartTag(_) | Token::EndTag(_) => return Some(Ok(token)),
                        _ => panic!("unexpected token")
                    };
                },
                Some('\0') => {
                    // TODO: Parse error: "unexpected-null-character"
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            tag.name.push('\u{fffd}');
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                Some(c) => {
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            tag.name.push(c.to_ascii_lowercase());
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                None => {
                    // TODO: Parse error: "eof-in-tag"
                    return None;
                }
            },
            TokenizerState::RcdataLessThanSign => todo!(),
            TokenizerState::RcdataEndTagOpen => todo!(),
            TokenizerState::RcdataEndTagName => todo!(),
            TokenizerState::RawtextLessThanSign => todo!(),
            TokenizerState::RawtextEndTagOpen => todo!(),
            TokenizerState::RawtextEndTagName => todo!(),
            TokenizerState::ScriptDataLessThanSign => todo!(),
            TokenizerState::ScriptDataEndTagOpen => todo!(),
            TokenizerState::ScriptDataEndTagName => todo!(),
            TokenizerState::ScriptDataEscapeStart => todo!(),
            TokenizerState::ScriptDataEscapeStartDash => todo!(),
            TokenizerState::ScriptDataEscaped => todo!(),
            TokenizerState::ScriptDataEscapedDash => todo!(),
            TokenizerState::ScriptDataEscapedDashDash => todo!(),
            TokenizerState::ScriptDataEscapedLessThanSign => todo!(),
            TokenizerState::ScriptDataEscapedEndTagOpen => todo!(),
            TokenizerState::ScriptDataEscapedEndTagName => todo!(),
            TokenizerState::ScriptDataDoubleEscapeStart => todo!(),
            TokenizerState::ScriptDataDoubleEscaped => todo!(),
            TokenizerState::ScriptDataDoubleEscapedDash => todo!(),
            TokenizerState::ScriptDataDoubleEscapedDashDash => todo!(),
            TokenizerState::ScriptDataDoubleEscapedLessThanSign => todo!(),
            TokenizerState::ScriptDataDoubleEscapeEnd => todo!(),
            TokenizerState::BeforeAttributeName => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    return None;
                },
                Some('/') | Some('>') | None => {
                    self.state = TokenizerState::AfterAttributeName;
                    return self.parse_character(current_input_character, parser);
                },
                Some('=') => {
                    // TODO: Parse error: "unexpected-equals-sign-before-attribute-name"
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag)) | Some(Token::EndTag(ref mut tag)) => {
                            let mut attribute = Attribute::new(self.allocator);
                            attribute.name.push('=');
                            tag.attributes.push(attribute);
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                    self.state = TokenizerState::AttributeName;
                },
                Some(_) => {
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag)) | Some(Token::EndTag(ref mut tag)) => {
                            tag.attributes.push(Attribute::new(self.allocator));
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                    self.state = TokenizerState::AttributeName;
                    self.parse_character(current_input_character, parser);
                }
            },
            TokenizerState::AttributeName => match current_input_character {
                Some('\t') | Some('\n') | Some('\x0c') | Some(' ') | Some('/') | Some('>') | None => {
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            let name = &tag.attributes.last().expect("no attributes").name;
                            self.discarding_attribute =
                                tag.attributes[0 .. tag.attributes.len() - 1].iter().any(|attr| attr.name == *name);
                            if self.discarding_attribute {
                                // TODO: Parse error: "duplicate-attribute"
                            }
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                    self.state = TokenizerState::AfterAttributeName;
                    return self.parse_character(current_input_character, parser);
                },
                Some('=') => {
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            let name = &tag.attributes.last().expect("no attributes").name;
                            self.discarding_attribute =
                                tag.attributes[0 .. tag.attributes.len() - 1].iter().any(|attr| attr.name == *name);
                            if self.discarding_attribute {
                                // TODO: Parse error: "duplicate-attribute"
                            }
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                    self.state = TokenizerState::BeforeAttributeValue;
                },
                Some('\0') => {
                    // TODO: Parse error: "unexpected-null-character"
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            tag.attributes.last_mut().expect("no attributes").name.push('\u{fffd}');
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                Some(c) => {
                    if ['"', '\'', '<'].contains(&c) {
                        // TODO: Parse error: "unexpected-character-in-attribute-name"
                    }
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            tag.attributes.last_mut().expect("no attributes").name.push(c.to_ascii_lowercase());
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                }
            },
            TokenizerState::AfterAttributeName => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    return None;
                },
                Some('/') => {
                    self.state = TokenizerState::SelfClosingStartTag;
                },
                Some('=') => {
                    self.state = TokenizerState::BeforeAttributeValue;
                },
                Some('>') => {
                    self.state = TokenizerState::Data;
                    let mut token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag) => {
                            if self.discarding_attribute {
                                tag.attributes.pop();
                            }
                            return Some(Ok(token));
                        },
                        _ => panic!("unexpected token")
                    };
                },
                Some(c) => {
                    self.state = TokenizerState::AttributeName;
                    return self.parse_character(current_input_character, parser);
                },
                None => {
                    // TODO: Parse error: "eof-in-tag"
                    return None;
                }
            },
            TokenizerState::BeforeAttributeValue => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    return None;
                },
                Some('"') => {
                    self.state = TokenizerState::AttributeValueQuoted('"');
                },
                Some('\'') => {
                    self.state = TokenizerState::AttributeValueQuoted('\'');
                },
                Some('>') => {
                    // TODO: Parse error: "missing-attribute-value"
                    self.state = TokenizerState::Data;
                    let mut token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag) => {
                            if self.discarding_attribute {
                                tag.attributes.pop();
                            }
                            return Some(Ok(token));
                        },
                        _ => panic!("unexpected token")
                    };
                },
                Some(_) | None => {
                    self.state = TokenizerState::AttributeValueUnquoted;
                    return self.parse_character(current_input_character, parser);
                }
            },
            // "Attribute value (double-quoted) state" and "Attribute value (single-quoted) state"
            TokenizerState::AttributeValueQuoted(quote) => match current_input_character {
                Some(c) if c == quote => {
                    self.state = TokenizerState::AfterAttributeValueQuoted;
                },
                Some('&') => {
                    assert!(self.return_state.is_none());
                    self.return_state = Some(self.state);
                    self.state = TokenizerState::CharacterReference;
                },
                Some('\0') => {
                    // TODO: Parse error: "unexpected-null-character"
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            tag.attributes.last_mut().expect("no attributes").value.push('\u{fffd}');
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                Some(c) => {
                    match self.current_token {
                        Some(Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag)) => {
                            tag.attributes.last_mut().expect("no attributes").value.push(c);
                        },
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                None => {
                    // TODO: Parse error: "eof-in-tag"
                }
            },
            TokenizerState::AttributeValueUnquoted => todo!(),
            TokenizerState::AfterAttributeValueQuoted => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    self.state = TokenizerState::BeforeAttributeName;
                },
                Some('/') => {
                    self.state = TokenizerState::SelfClosingStartTag;
                },
                Some('>') => {
                    self.state = TokenizerState::Data;
                    let mut token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag) => {
                            if self.discarding_attribute {
                                tag.attributes.pop();
                            }
                            return Some(Ok(token));
                        },
                        _ => panic!("unexpected token")
                    };
                },
                Some(_) => {
                    // TODO: Parse error: "missing-whitespace-between-attributes"
                    self.state = TokenizerState::BeforeAttributeName;
                    return self.parse_character(current_input_character, parser);
                },
                None => {
                    // TODO: Parse error: "eof-in-tag"
                    return None;
                }
            },
            TokenizerState::SelfClosingStartTag => match current_input_character {
                Some('>') => {
                    self.state = TokenizerState::Data;
                    let mut token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::StartTag(ref mut tag) | Token::EndTag(ref mut tag) => {
                            if self.discarding_attribute {
                                tag.attributes.pop();
                            }
                            tag.self_closing = true;
                            return Some(Ok(token));
                        },
                        _ => panic!("unexpected token")
                    };
                },
                Some(_) => {
                    // TODO: Parse error: "unexpected-solidus-in-tag"
                    self.state = TokenizerState::BeforeAttributeName;
                    return self.parse_character(current_input_character, parser);
                },
                None => {
                    // TODO: Parse error: "eof-in-tag"
                    return None;
                }
            },
            TokenizerState::BogusComment => match current_input_character {
                Some('>') => {
                    self.state = TokenizerState::Data;
                    let token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::Comment(_) => return Some(Ok(token)),
                        _ => panic!("unexpected token")
                    };
                },
                Some('\0') => {
                    // TODO: Parse error: "unexpected-null-character"
                    match self.current_token {
                        Some(Token::Comment(ref mut comment)) => comment.push('\u{fffd}'),
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                Some(c) => {
                    match self.current_token {
                        Some(Token::Comment(ref mut comment)) => comment.push(c),
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                None => {
                    let token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    return Some(Ok(token));
                }
            },
            TokenizerState::MarkupDeclarationOpen => match current_input_character {
                Some('-') => {
                    match self.temporary_buffer.as_str() {
                        "" => {
                            self.temporary_buffer.push('-');
                        },
                        "-" => {
                            self.temporary_buffer.clear();
                            self.current_token = Some(Token::Comment(
                                String::new_in(self.allocator)
                            ));
                            self.state = TokenizerState::CommentStart;
                        },
                        _ => {
                            let comment = mem::replace(
                                &mut self.temporary_buffer,
                                String::new_in(self.allocator)
                            );
                            assert!(self.current_token.is_none(), "unexpected token");
                            self.current_token = Some(Token::Comment(comment));
                            self.state = TokenizerState::BogusComment;
                            return self.parse_character(current_input_character, parser);
                        }
                    };
                },
                Some(c) => {
                    self.temporary_buffer.push(c);
                    if "DOCTYPE"[0 .. usize::min(self.temporary_buffer.len(), "DOCTYPE".len())]
                            .eq_ignore_ascii_case(self.temporary_buffer.as_str()) {
                        if self.temporary_buffer.len() == "DOCTYPE".len() {
                            // Found "DOCTYPE" (case-insensitive).
                            self.temporary_buffer.clear();
                            self.state = TokenizerState::Doctype;
                        }
                    } else if "[CDATA[".starts_with(self.temporary_buffer.as_str()) {
                        if self.temporary_buffer.len() == "[CDATA[".len() {
                            // Found "[CDATA[".
                            match parser.adjusted_current_node() {
                                Some(node) if node.borrow().as_elem().identifier.namespace != namespace::HTML => {
                                    self.temporary_buffer.clear();
                                    self.state = TokenizerState::CdataSection;
                                },
                                _ => {
                                    // TODO: Parse error: "cdata-in-html-content"
                                    let comment = mem::replace(
                                        &mut self.temporary_buffer,
                                        String::new_in(self.allocator)
                                    );
                                    assert!(self.current_token.is_none(), "unexpected token");
                                    self.current_token = Some(Token::Comment(comment));
                                    self.state = TokenizerState::BogusComment;
                                }
                            };
                        }
                    } else {
                        // TODO: Parse error: "incorrectly-opened-comment"
                        self.temporary_buffer.pop();
                        let comment = mem::replace(
                            &mut self.temporary_buffer,
                            String::new_in(self.allocator)
                        );
                        assert!(self.current_token.is_none(), "unexpected token");
                        self.current_token = Some(Token::Comment(comment));
                        self.state = TokenizerState::BogusComment;
                        return self.parse_character(current_input_character, parser);
                    }
                },
                None => {
                    // TODO: Parse error: "incorrectly-opened-comment"
                    self.state = TokenizerState::BogusComment;
                    return self.parse_character(current_input_character, parser);
                }
            },
            TokenizerState::CommentStart => todo!(),
            TokenizerState::CommentStartDash => todo!(),
            TokenizerState::Comment => todo!(),
            TokenizerState::CommentLessThanSign => todo!(),
            TokenizerState::CommentLessThanSignBang => todo!(),
            TokenizerState::CommentLessThanSignBangDash => todo!(),
            TokenizerState::CommentLessThanSignBangDashDash => todo!(),
            TokenizerState::CommentEndDash => todo!(),
            TokenizerState::CommentEnd => todo!(),
            TokenizerState::CommentEndBang => todo!(),
            TokenizerState::Doctype => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    self.state = TokenizerState::BeforeDoctypeName;
                },
                Some('>') => {
                    self.state = TokenizerState::BeforeDoctypeName;
                    return self.parse_character(current_input_character, parser);
                },
                Some(_) => {
                    // TODO: Parse error: "missing-whitespace-before-doctype-name"
                    self.state = TokenizerState::BeforeDoctypeName;
                    return self.parse_character(current_input_character, parser);
                },
                None => {
                    let doctype = Doctype {
                        force_quirks: true,
                        ..Doctype::new()
                    };
                    return Some(Err((Token::Doctype(doctype), ParseError::EofInDoctype)));
                }
            },
            TokenizerState::BeforeDoctypeName => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    // Ignore the character.
                },
                Some('>') => {
                    let doctype = Doctype {
                        force_quirks: true,
                        ..Doctype::new()
                    };
                    self.state = TokenizerState::Data;
                    return Some(Err((Token::Doctype(doctype), ParseError::MissingDoctypeName)));
                }
                Some(c) => {
                    let doctype = Doctype {
                        name: Some(String::new_in(self.allocator)),
                        ..Doctype::new()
                    };
                    assert!(self.current_token.is_none(), "unexpected token");
                    self.current_token = Some(Token::Doctype(doctype));
                    self.state = TokenizerState::DoctypeName;
                    return self.parse_character(current_input_character, parser);
                },
                None => {
                    let doctype = Doctype {
                        force_quirks: true,
                        ..Doctype::new()
                    };
                    return Some(Err((Token::Doctype(doctype), ParseError::EofInDoctype)));
                }
            },
            TokenizerState::DoctypeName => match current_input_character {
                Some(c) if c.is_ascii_whitespace() => {
                    self.state = TokenizerState::AfterDoctypeName;
                },
                Some('>') => {
                    self.state = TokenizerState::Data;
                    let token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::Doctype(_) => return Some(Ok(token)),
                        _ => panic!("unexpected token")
                    };
                },
                Some('\0') => {
                    // TODO: Parse error: "unexpected-null-character"
                    match self.current_token {
                        Some(Token::Doctype(Doctype { name: Some(ref mut name), .. })) => name.push('\u{fffd}'),
                        Some(Token::Doctype(Doctype { name: None, .. })) => panic!("uninitialized DOCTYPE name"),
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                Some(c) => {
                    match self.current_token {
                        Some(Token::Doctype(Doctype { name: Some(ref mut name), .. })) => name.push(c.to_ascii_lowercase()),
                        Some(Token::Doctype(Doctype { name: None, .. })) => panic!("uninitialized DOCTYPE name"),
                        Some(_) => panic!("unexpected token"),
                        None => panic!("no current token")
                    };
                },
                None => {
                    let token = mem::replace(&mut self.current_token, None)
                        .expect("no current token");
                    match token {
                        Token::Doctype(_) => return Some(Err((token, ParseError::EofInDoctype))),
                        _ => panic!("unexpected token")
                    };
                }
            },
            TokenizerState::AfterDoctypeName => todo!(),
            TokenizerState::AfterDoctypePublicKeyword => todo!(),
            TokenizerState::BeforeDoctypePublicIdentifier => todo!(),
            TokenizerState::DoctypePublicIdentifierDoubleQuoted => todo!(),
            TokenizerState::DoctypePublicIdentifierSingleQuoted => todo!(),
            TokenizerState::AfterDoctypePublicIdentifier => todo!(),
            TokenizerState::BetweenDoctypePublicAndSystemIdentifiers => todo!(),
            TokenizerState::AfterDoctypeSystemKeyword => todo!(),
            TokenizerState::BeforeDoctypeSystemIdentifier => todo!(),
            TokenizerState::DoctypeSystemIdentifierDoubleQuoted => todo!(),
            TokenizerState::DoctypeSystemIdentifierSingleQuoted => todo!(),
            TokenizerState::AfterDoctypeSystemIdentifier => todo!(),
            TokenizerState::BogusDoctype => todo!(),
            TokenizerState::CdataSection => todo!(),
            TokenizerState::CdataSectionBracket => todo!(),
            TokenizerState::CdataSectionEnd => todo!(),
            TokenizerState::CharacterReference => todo!(),
            TokenizerState::NamedCharacterReference => todo!(),
            TokenizerState::AmbiguousAmpersand => todo!(),
            TokenizerState::NumericCharacterReference => todo!(),
            TokenizerState::HexadecimalCharacterReferenceStart => todo!(),
            TokenizerState::DecimalCharacterReferenceStart => todo!(),
            TokenizerState::HexadecimalCharacterReference => todo!(),
            TokenizerState::DecimalCharacterReference => todo!(),
            TokenizerState::NumericCharacterReferenceEnd => todo!(),

            TokenizerState::EmitCharacters(s) => match s.chars().next() {
                Some(c) => {
                    self.state = TokenizerState::EmitCharacters(s.strip_prefix(c).unwrap());
                    return Some(Ok(Token::Character(c)));
                },
                None => {
                    return None;
                }
            }
        };

        None
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TokenizerState {
    Data,
    Rcdata,
    Rawtext,
    ScriptData,
    Plaintext,
    TagOpen,
    EndTagOpen,
    TagName,
    RcdataLessThanSign,
    RcdataEndTagOpen,
    RcdataEndTagName,
    RawtextLessThanSign,
    RawtextEndTagOpen,
    RawtextEndTagName,
    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueQuoted(char),
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,
    CdataSection,
    CdataSectionBracket,
    CdataSectionEnd,
    CharacterReference,
    NamedCharacterReference,
    AmbiguousAmpersand,
    NumericCharacterReference,
    HexadecimalCharacterReferenceStart,
    DecimalCharacterReferenceStart,
    HexadecimalCharacterReference,
    DecimalCharacterReference,
    NumericCharacterReferenceEnd,

    // Not described by the spec, this state just emits the given characters one at a time.
    EmitCharacters(&'static str)
}

#[derive(Debug)]
pub(super) enum Token<A: alloc::alloc::Allocator+Copy> {
    Doctype(Doctype<A>),
    StartTag(Tag<A>),
    EndTag(Tag<A>),
    Comment(String<A>),
    Character(char),
    EndOfFile
}

impl<A: alloc::alloc::Allocator+Copy> Token<A> {
    pub fn is_eof(&self) -> bool {
        if let Self::EndOfFile = *self {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub(super) struct Doctype<A: alloc::alloc::Allocator> {
    pub name: Option<String<A>>,
    pub public_identifier: Option<String<A>>,
    pub system_identifier: Option<String<A>>,
    pub force_quirks: bool
}

impl<A: alloc::alloc::Allocator> Doctype<A> {
    fn new() -> Self {
        Self {
            name: None,
            public_identifier: None,
            system_identifier: None,
            force_quirks: false
        }
    }
}

#[derive(Debug)]
pub(super) struct Tag<A: alloc::alloc::Allocator+Copy> {
    pub name: InternedString<A>,
    pub self_closing: bool,
    pub attributes: Vec<Attribute<A>, A>
}

impl<A: alloc::alloc::Allocator+Copy> Tag<A> {
    pub(super) fn new(allocator: A) -> Self {
        Self {
            name: InternedString::new_in(allocator),
            self_closing: false,
            attributes: Vec::new_in(allocator)
        }
    }
}
