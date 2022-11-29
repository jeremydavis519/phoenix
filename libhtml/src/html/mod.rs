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

//! This module defines the HTML parser.

// Comments consisting entirely of quotations come directly from the HTML specification and serve as
// bookmarks in that searching for them word-for-word will lead to the relevant part of the spec.

pub(super) mod decoder;
pub        mod dom;
pub        mod element;
pub        mod encoding;
pub(super) mod node;
           mod prescan;
pub(super) mod tokenizer;

use {
    alloc::{
        boxed::Box,
        collections::vec_deque::VecDeque,
        rc::{Rc, Weak},
        vec::Vec
    },
    core::cell::RefCell,
    decoder::DecodeResult,
    element::{Element, Identifier},
    encoding::{CharEncoding, CharEncodingConfidence, MaybeCharEncoding},
    node::{Comment, DocumentType, Node},
    tokenizer::{Doctype, Token, Tag, Tokenizer, TokenizerState},
    crate::shim::String,
    crate::{
        interned_string::InternedString,
        namespace
    },
    super::{ByteDocumentInternal, CharDocumentInternal, DocumentInternal, QuirksMode}
};

#[derive(Debug)]
pub(super) struct Parser<A: alloc::alloc::Allocator+Copy> {
    allocator: A,

    // https://html.spec.whatwg.org/multipage/parsing.html#insertion-mode
    insertion_mode: InsertionMode,
    // https://html.spec.whatwg.org/multipage/parsing.html#original-insertion-mode
    original_insertion_mode: Option<InsertionMode>,
    // The stack of insertion modes for <template> elements
    template_insertion_modes: Vec<InsertionMode, A>,
    
    parser_cannot_change_the_mode: bool,

    // https://html.spec.whatwg.org/multipage/parsing.html#stack-of-open-elements
    open_elements: Vec<Rc<RefCell<Node<A>>>, A>,

    // "foster parenting"
    foster_parenting_enabled: bool,

    frameset_ok: bool,

    // "scripting flag"
    scripting: bool,

    script_nesting_level: usize,
    parser_paused: bool,

    html_fragment_parsing_algorithm_context: Option<Rc<RefCell<Node<A>>>>,

    // Indicates that the next token will be ignored if it is a '\n' character token.
    skip_newline: bool,

    // https://html.spec.whatwg.org/multipage/parsing.html#active-speculative-html-parser
    active_speculative_html_parser: Option<Box<Parser<A>, A>>
}

impl<A: alloc::alloc::Allocator+Copy> Parser<A> {
    /// Creates a new parser.
    pub fn new(scripting: bool, allocator: A) -> Self {
        Self {
            allocator,

            insertion_mode: InsertionMode::Initial,

            original_insertion_mode: None,
            template_insertion_modes: Vec::new_in(allocator),

            parser_cannot_change_the_mode: false,

            open_elements: Vec::new_in(allocator),

            foster_parenting_enabled: false,

            frameset_ok: true,

            scripting,

            script_nesting_level: 0,
            parser_paused: false,

            html_fragment_parsing_algorithm_context: None,

            skip_newline: false,

            active_speculative_html_parser: None
        }
    }

    /// Attempts to parse and consume all the bytes currently in the queue. Any number of bytes up
    /// to the current size of the queue may be consumed, including zero. Generally, if the end of
    /// the stream is reached, then this function is called, and there are still bytes in the queue
    /// at the end of that process, an unresolvable parsing error has occurred.
    pub fn flush_byte_stream(
            &mut self,
            tokenizer: &mut Tokenizer<A>,
            document: &mut ByteDocumentInternal<A>,
            eof: bool
    ) {
        const MAX_CHAR_LENGTH: usize = 4;

        // If we don't know the character encoding, try to determine it.
        if document.internal.encoding.is_none() {
            match prescan::sniff_char_encoding(document) {
                ParseResult::Ok((encoding, confidence)) => {
                    document.internal.encoding = Some(encoding);
                    document.internal.enc_confidence = confidence;
                },
                ParseResult::Later => {
                    // As long as we don't know the encoding, we can't do anything.
                    return;
                },
                ParseResult::Err(_) => unreachable!()
            };
        }

        // Decode the byte stream using the BOM encoding if possible. For backward compatibility,
        // the spec distinguishes between the document's encoding and the encoding used for
        // decoding. For the latter, the BOM trumps everything.
        *tokenizer.encoding.borrow_mut() = if let MaybeCharEncoding::Some(bom_encoding) = document.bom_encoding {
            bom_encoding
        } else {
            document.internal.encoding.unwrap()
        };
        let mut buffer = [0u8; MAX_CHAR_LENGTH];
        let mut buffer_size = 0;
        let mut bytes = PopFrontIterator::new(&mut document.byte_queue);

        loop {
            let encoding = *tokenizer.encoding.borrow();
            let mut code_points = (&mut bytes)
                .filter_map(|b| {
                    buffer[buffer_size] = b;
                    buffer_size += 1;
                    match decoder::decode(&buffer, encoding) {
                        DecodeResult::Ok(c, bytes_used) => {
                            buffer.copy_within(bytes_used .. buffer_size, 0);
                            buffer_size -= bytes_used;
                            Some(c)
                        },
                        DecodeResult::Err(parse_error, bytes_used) => {
                            // TODO: Log an error message with the error and its location.
                            buffer.copy_within(bytes_used .. buffer_size, 0);
                            buffer_size -= bytes_used;
                            None
                        },
                        DecodeResult::Incomplete => None
                    }
                });

            let token = match tokenizer.tokenize(&mut code_points, self, eof) {
                Some(Ok(token)) => token,
                Some(Err((token, parse_error))) => {
                    // TODO: Log an error message with the error and its location.
                    token
                },
                None => break
            };

            self.parse_token(token, tokenizer, self.insertion_mode, &mut document.internal);
        }

        // Put any partial token that might remain back onto the byte queue so it can be properly
        // tokenized at the next flush.
        for &byte in buffer[0 .. buffer_size].iter().rev() {
            document.byte_queue.push_front(byte);
        }
    }

    /// Attempts to parse and consume all the code points currently in the queue. Any number of code
    /// points up to the current size of the queue may be consumed, including zero. Generally, if
    /// the end of the stream is reached, then this function is called, and there are still code
    /// points in the queue at the end of that process, an unresolvable parsing error has occurred.
    pub fn flush_char_stream(
            &mut self,
            tokenizer: &mut Tokenizer<A>,
            document: &mut CharDocumentInternal<A>,
            eof: bool
    ) {
        let mut code_points = PopFrontIterator::new(&mut document.char_queue);
        while let Some(token_result) = tokenizer.tokenize(&mut code_points, self, eof) {
            let token = match token_result {
                Ok(token) => token,
                Err((token, parse_error)) => {
                    // TODO: Log an error message with the error and its location.
                    token
                }
            };

            self.parse_token(token, tokenizer, self.insertion_mode, &mut document.internal);
        }
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#tree-construction-dispatcher
    fn parse_token(
            &mut self,
            token: Token<A>,
            tokenizer: &mut Tokenizer<A>,
            insertion_mode: InsertionMode,
            document: &mut DocumentInternal<A>
    ) {
        // Some of the rules below say to skip the next token if it is U+000A LINE FEED (LF).
        if self.skip_newline {
            if let Token::Character('\n') = token {
                return;
            }
            self.skip_newline = false;
        }

        let mut is_html = false;

        // "If the stack of open elements is empty"
        if self.open_elements.is_empty() { is_html = true; }

        else if let Some(node) = self.adjusted_current_node() {
            match *node.borrow() {
                // "If the adjusted current node is an element in the HTML namespace"
                Node::Element(ref elem) if elem.identifier.namespace == namespace::HTML => is_html = true,

                Node::Element(ref elem) if elem.is_mathml_text_integration_point() => {
                    match token {
                        // "If the adjusted current node is a MathML text integration point and the token is a start tag
                        // whose tag name is neither "mglyph" nor "malignmark""
                        Token::StartTag(ref tag) if !["mglyph", "malignmark"].contains(&tag.name.as_str()) =>
                            is_html = true,

                        // "If the adjusted current node is a MathML text integration point and the token is a character
                        // token"
                        Token::Character(_) => is_html = true,

                        _ => {}
                    };
                },

                Node::Element(ref elem)
                        if elem.identifier.namespace == namespace::MATHML
                            && elem.identifier.local_name == "annotation-xml" => {
                    match token {
                        // "If the adjusted current node is a MathML annotation-xml element and the token is a start tag
                        // whose tag name is "svg""
                        Token::StartTag(ref tag) if tag.name == "svg" => is_html = true,

                        _ => {}
                    };
                },

                Node::Element(ref elem) if elem.is_html_integration_point() => {
                    match token {
                        // "If the adjusted current node is an HTML integration point and the token is a start tag"
                        Token::StartTag(_) => is_html = true,

                        // "If the adjusted current node is an HTML integration point and the token is a character token"
                        Token::Character(_) => is_html = true,

                        _ => {}
                    };
                },

                _ => {}
            };
        }

        // "If the token is an end-of-file token"
        if is_html || token.is_eof() {
            // "Process the token according to the rules given in the section corresponding to the current insertion
            // mode in HTML content."
            return self.parse_html_token(token, tokenizer, insertion_mode, document);
        }

        // "Otherwise"
        // TODO: "Process the token according to the rules given in the section for parsing tokens in foreign content."
        todo!()
    }    

    // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhtml
    fn parse_html_token(
            &mut self,
            mut token: Token<A>,
            tokenizer: &mut Tokenizer<A>,
            insertion_mode: InsertionMode,
            document: &mut DocumentInternal<A>
    ) {
        match insertion_mode {
            // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
            InsertionMode::Initial => match token {
                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF),
                // U+000C FORM FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Ignore the token."
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment as the last child of the Document object."
                    let index = document.dom.children.len();
                    Self::insert_comment(comment, &mut document.dom.children, index);
                },

                // "A DOCTYPE token"
                Token::Doctype(doctype) => {
                    // FIXME: Where is this in the specification?
                    let mut set_quirks_mode = QuirksMode::NoQuirks;

                    // "If the DOCTYPE token's name is not "html", or the token's public identifier is not
                    // missing, or the token's system identifier is neither missing nor "about:legacy-compat",
                    // then there is a parse error."
                    match doctype.name {
                        None => {
                            // TODO: Parse error: "missing-doctype-name"
                            set_quirks_mode = QuirksMode::Quirks;
                        },
                        Some(ref name) if name != "html" => {
                            // TODO: Parse error: non-HTML DOCTYPE
                            set_quirks_mode = QuirksMode::Quirks;
                        },
                        Some(_) => {}
                    };
                    match doctype.public_identifier {
                        None => {},
                        Some(ref public_identifier) => {
                            // TODO: Parse error: public identifier is not missing
                            match self.get_quirks_mode_from_public_identifier(
                                    public_identifier,
                                    doctype.system_identifier.is_some()
                            ) {
                                QuirksMode::Quirks => set_quirks_mode = QuirksMode::Quirks,
                                QuirksMode::LimitedQuirks => set_quirks_mode = QuirksMode::LimitedQuirks,
                                QuirksMode::NoQuirks => {}
                            };
                        }
                    };
                    match doctype.system_identifier {
                        None => {},
                        Some(ref system_identifier) => {
                            if !system_identifier.eq_ignore_ascii_case("about:legacy-compat") {
                                // TODO: Parse error
                                if system_identifier.eq_ignore_ascii_case(
                                    "http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd"
                                ) {
                                    set_quirks_mode = QuirksMode::Quirks
                                }
                            }
                        }
                    }

                    // "Append a DocumentType node to the Document node, with its name set to the name given
                    // in the DOCTYPE token, or the empty string if the name was missing; its public ID set to
                    // the public identifier given in the DOCTYPE token, or the empty string if the public
                    // identifier was missing; and its system ID set to the system identifier given in the
                    // DOCTYPE token, or the empty string if the system identifier was missing."
                    self.insert_doctype(document, doctype);

                    // "Then, if the document is not an iframe srcdoc document, and the parser cannot change the
                    // mode flag is false, and the DOCTYPE token matches one of the conditions in the following
                    // list, then set the Document to quirks mode: ..."
                    // "Otherwise, if the document is not an iframe srcdoc document, and the parser cannot change
                    // the mode flag is false, and the DOCTYPE token matches one of the conditions in the following
                    // list, then then set the Document to limited-quirks mode: ..."
                    // "The system identifier and public identifier strings must be compared to the values given in
                    // the lists above in an ASCII case-insensitive manner. A system identifier whose value is the
                    // empty string is not considered missing for the purposes of the conditions above."
                    // NOTE: Most of these checks are actually done before this point.
                    if !document.is_iframe_srcdoc && !self.parser_cannot_change_the_mode {
                        document.quirks_mode = set_quirks_mode;
                    }

                    // "Then, switch the insertion mode to "before html"."
                    self.insertion_mode = InsertionMode::BeforeHtml;
                },

                // "Anything else"
                token => {
                    // "If the document is not an iframe srcdoc document, then this is a parse error; if the parser
                    // cannot change the mode flag is false, set the Document to quirks mode."
                    if !document.is_iframe_srcdoc {
                        // TODO: Parse error
                        if !self.parser_cannot_change_the_mode {
                            document.quirks_mode = QuirksMode::Quirks;
                        }
                    }

                    // "In any case, switch the insertion mode to "before html", then reprocess the token."
                    self.insertion_mode = InsertionMode::BeforeHtml;
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#the-before-html-insertion-mode
            InsertionMode::BeforeHtml => match token {
                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment as the last child of the Document object."
                    let index = document.dom.children.len();
                    Self::insert_comment(comment, &mut document.dom.children, index);
                },

                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Ignore the token."
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(tag) if tag.name == "html" => {
                    // "Create an element for the token in the HTML namespace, with the Document as the intended
                    // parent. Append it to the Document object. Put this element in the stack of open elements."
                    // FIXME: The document, not null (Weak::new()) should be the intended parent.
                    let element = self.element_from_tag(document, tag, namespace::HTML, Weak::new());
                    document.dom.children.push(element.clone());
                    self.open_elements.push(element);

                    // "Switch the insertion mode to "before head"."
                    self.insertion_mode = InsertionMode::BeforeHead;
                },

                // "Any other end tag"
                // NOTE: This is out of order here. It means the tag name is not "head", "body", "html", or "br".
                Token::EndTag(tag)
                        if !["head", "body", "html", "br"].contains(&tag.name.as_str()) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "An end tag whose tag name is one of: "head", "body", "html", "br""
                // "Anything else"
                token => {
                    // "Create an html element whose node document is the Document object. Append it to the Document
                    // object. Put this element in the stack of open elements."
                    let tag = Tag {
                        name: InternedString::from_in("html", self.allocator),
                        ..Tag::new(self.allocator)
                    };
                    // FIXME: The document, not null (Weak::new()) should be the intended parent.
                    let element = self.element_from_tag(document, tag, namespace::HTML, Weak::new());
                    document.dom.children.push(element.clone());
                    self.open_elements.push(element);

                    // "Switch the insertion mode to "before head", then reprocess the token."
                    self.insertion_mode = InsertionMode::BeforeHead;
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#the-before-head-insertion-mode
            InsertionMode::BeforeHead => match token {
                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Ignore the token."
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment."
                    let (parent, index) = self.appropriate_place_for_inserting_a_node(
                        self.current_node().expect("no current node")
                    );
                    Self::insert_comment(comment, parent.borrow_mut().children_mut(), index);
                },

                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(ref tag) if tag.name == "html" => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "A start tag whose tag name is "head""
                Token::StartTag(tag) if tag.name == "head" => {
                    // "Insert an HTML element for the token."
                    let element = self.insert_html_element(document, tag);

                    // "Set the head element pointer to the newly created head element."
                    document.head_element = Some(element.clone());

                    // "Switch the insertion mode to "in head"."
                    self.insertion_mode = InsertionMode::InHead;
                },

                // "Any other end tag"
                // NOTE: This is out of order here. It means the tag name is not "head", "body", "html", or "br".
                Token::EndTag(tag)
                        if !["head", "body", "html", "br"].contains(&tag.name.as_str()) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "An end tag whose tag name is one of: "head", "body", "html", "br""
                // "Anything else"
                token => {
                    // "Insert an HTML element for a "head" start tag token with no attributes."
                    let tag = Tag {
                        name: InternedString::from_in("head", self.allocator),
                        ..Tag::new(self.allocator)
                    };
                    let element = self.insert_html_element(document, tag);

                    // "Set the head element pointer to the newly created head element."
                    document.head_element = Some(element.clone());

                    // "Switch the insertion mode to "in head"."
                    self.insertion_mode = InsertionMode::InHead;

                    // "Reprocess the current token."
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inhead
            InsertionMode::InHead => match token {
                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Insert the character."
                    self.insert_character(c);
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment."
                    let (parent, index) = self.appropriate_place_for_inserting_a_node(
                        self.current_node().expect("no current node")
                    );
                    Self::insert_comment(comment, parent.borrow_mut().children_mut(), index);
                },

                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(ref tag) if tag.name == "html" => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "A start tag whose tag name is one of: "base", "basefont", "bgsound", "link""
                Token::StartTag(tag)
                        if ["base", "basefont", "bgsound", "link"].contains(&tag.name.as_str()) => {
                    // "Insert an HTML element for the token. Immediately pop the current node off the stack
                    // of open elements."
                    self.insert_html_element(document, tag);
                    let element = self.open_elements.pop().expect("no current node");

                    // "Acknowledge the token's self-closing flag, if it is set."
                    element.borrow_mut().acknowledge_self_closing();
                },

                // "A start tag whose tag name is "meta""
                Token::StartTag(tag) if tag.name == "meta" => {
                    // "Insert an HTML element for the token. Immediately pop the current node off the stack
                    // of open elements."
                    self.insert_html_element(document, tag);
                    let element = self.open_elements.pop().expect("no current node");

                    // "Acknowledge the token's self-closing flag, if it is set."
                    element.borrow_mut().acknowledge_self_closing();

                    // "If the active speculative HTML parser is null, then:"
                    if self.active_speculative_html_parser.is_none() {
                        if document.enc_confidence == CharEncodingConfidence::Tentative {
                            // "1. If the element has a charset attribute, and getting an encoding from its value
                            // results in an encoding, and the confidence is currently tentative, then change the
                            // encoding to the resulting encoding."
                            if let Some(attr) = element.borrow().as_elem().attributes.iter()
                                    .find(|attr| attr.name == "charset") {
                                if let Ok(encoding) = CharEncoding::from_label(attr.value.as_bytes()) {
                                    Self::change_encoding(encoding, tokenizer, document);
                                    return;
                                }
                            }

                            // TODO: "2. Otherwise, if the element has an http-equiv attribute whose value is an ASCII
                            // case-insensitive match for the string "Content-Type", and the element has a content
                            // attribute, and applying the algorithm for extracting a character encoding from a meta
                            // element to that attribute's value returns an encoding, and the confidence is currently
                            // tentative, then change the encoding to the extracted encoding."
                            todo!()
                        }
                    }
                },

                // "A start tag whose tag name is "title""
                Token::StartTag(tag) if tag.name == "title" => {
                    // "Follow the generic RCDATA element parsing algorithm."
                    self.parse_text_element(tokenizer, TokenizerState::Rcdata, document, tag);
                },

                // "A start tag whose tag name is "noscript", if the scripting flag is enabled"
                // "A start tag whose tag name is one of: "noframes", "style""
                Token::StartTag(tag)
                        if (tag.name == "noscript" && self.scripting) ||
                            ["noframes", "style"].contains(&tag.name.as_str()) => {
                    // "Follow the generic raw text element parsing algorithm."
                    self.parse_text_element(tokenizer, TokenizerState::Rawtext, document, tag);
                },

                // "A start tag whose tag name is "noscript", if the scripting flag is disabled"
                Token::StartTag(tag)
                        if tag.name == "noscript" && !self.scripting => {
                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "Switch the insertion mode to "in head noscript"."
                    self.insertion_mode = InsertionMode::InHeadNoscript;
                },

                // "A start tag whose tag name is "script""
                Token::StartTag(tag) if tag.name == "script" => {
                    // "1. Let the adjusted insertion location be the appropriate place for inserting a node."
                    let (parent, insertion_index) = self.appropriate_place_for_inserting_a_node(
                        self.current_node().expect("no current node")
                    );

                    // "2. Create an element for the token in the HTML namespace, with the intended parent being
                    // the element in which the adjusted insertion location finds itself."
                    let element = self.element_from_tag(document, tag, namespace::HTML, Rc::downgrade(parent));
                    document.dom.children.push(element.clone());
                    self.open_elements.push(element);

                    // TODO
                    todo!()
                },

                // "An end tag whose tag name is "head""
                Token::EndTag(tag) if tag.name == "head" => {
                    // "Pop the current node (which will be the head element) off the stack of open elements."
                    let popped_element = self.open_elements.pop().expect("missing head element");
                    assert!(popped_element.borrow().as_elem().identifier == Identifier::new_html(
                        InternedString::from_in("head", self.allocator)
                    ));

                    // "Switch the insertion mode to "after head"."
                    self.insertion_mode = InsertionMode::AfterHead;
                },

                // "A start tag whose tag name is "template""
                Token::StartTag(tag) if tag.name == "template" => {
                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // TODO: "Insert a marker at the end of the list of active formatting elements."

                    // "Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;

                    // "Switch the insertion mode to "in template"."
                    self.insertion_mode = InsertionMode::InTemplate;

                    // "Push "in template" onto the stack of template insertion modes so that it is the new current
                    // template insertion mode."
                    self.template_insertion_modes.push(InsertionMode::InTemplate);
                },

                // "An end tag whose tag name is "template""
                Token::EndTag(tag) if tag.name == "template" => {
                    let tag_id = Identifier::new_html(tag.name);

                    // "If there is no template element on the stack of open elements, then this is a parse error;
                    // ignore the token."
                    if !self.open_elements.iter().any(|element| element.borrow().as_elem().identifier == tag_id) {
                        // TODO: Parse error.
                        return;
                    }

                    // "Otherwise, run these steps:"
                    // "1. Generate all implied end tags thoroughly."
                    self.generate_implied_end_tags_thoroughly();

                    // "2. If the current node is not a template element, then this is a parse error."
                    if self.current_node().expect("lost the template node").borrow().as_elem().identifier != tag_id {
                        // TODO: Parse error.
                    }

                    // "3. Pop elements from the stack of open elements until a template element has been popped from
                    // the stack."
                    while self.open_elements.pop().unwrap().borrow().as_elem().identifier != tag_id {}

                    // TODO: "4. Clear the list of active formatting elements up to the last marker."

                    // "5. Pop the current template insertion mode off the stack of template insertion modes."
                    self.template_insertion_modes.pop().expect("no template insertion modes");

                    // TODO: "6. Reset the insertion mode appropriately."
                    todo!();
                },

                // "A start tag whose tag name is "head""
                Token::StartTag(tag) if tag.name == "head" => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "Any other end tag"
                // NOTE: This is out of order here. It means the tag name is not "body", "html", or "br".
                Token::EndTag(tag)
                        if !["body", "html", "br"].contains(&tag.name.as_str()) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "An end tag whose tag name is one of: "body", "html", "br""
                // "Anything else"
                token => {
                    // "Pop the current node (which will be the head element) off the stack of open elements."
                    let popped_element = self.open_elements.pop().expect("missing head element");
                    assert!(popped_element.borrow().as_elem().identifier == Identifier::new_html(
                        InternedString::from_in("head", self.allocator)
                    ));

                    // "Switch the insertion mode to "after head"."
                    self.insertion_mode = InsertionMode::AfterHead;

                    // "Reprocess the token."
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inheadnoscript
            InsertionMode::InHeadNoscript => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#the-after-head-insertion-mode
            InsertionMode::AfterHead => match token {
                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Insert the character."
                    self.insert_character(c);
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment."
                    let (parent, index) = self.appropriate_place_for_inserting_a_node(
                        self.current_node().expect("no current node")
                    );
                    Self::insert_comment(comment, parent.borrow_mut().children_mut(), index);
                },

                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(ref tag) if tag.name == "html" => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "A start tag whose tag name is "body""
                Token::StartTag(tag) if tag.name == "body" => {
                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;

                    // "Switch the insertion mode to "in body"."
                    self.insertion_mode = InsertionMode::InBody;
                },

                // "A start tag whose tag name is "frameset""
                Token::StartTag(tag) if tag.name == "frameset" => {
                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "Switch the insertion mode to "in frameset"."
                    self.insertion_mode = InsertionMode::InFrameset;
                },

                // "A start tag whose tag name is one of: "base", "basefont", "bgsound", "link", "meta", "noframes",
                // "script", "style", "template", "title""
                Token::StartTag(ref tag)
                        if ["base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style",
                            "template", "title"].contains(&tag.name.as_str()) => {
                    // TODO: "Parse error"

                    // "Push the node pointed to by the head element pointer onto the stack of open elements."
                    self.open_elements.push(document.head_element.as_ref().expect("no head element").clone());

                    // "Process the token using the rules for the "in head" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InHead, document);

                    // "Remove the node pointed to by the head element pointer from the stack of open elements. (It
                    // might not be the current node at this point.)"
                    let head_element = document.head_element.as_ref().expect("no head element");
                    while !Rc::ptr_eq(
                                head_element,
                                &self.open_elements.pop().expect("head element not on stack of open elements")
                            ) {}
                },

                // "An end tag whose tag name is "template""
                Token::EndTag(ref tag) if tag.name == "template" => {
                    // "Process the token using the rules for the "in head" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InHead, document);
                },

                // "A start tag whose tag name is "head""
                Token::StartTag(tag) if tag.name == "head" => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "Any other end tag"
                // NOTE: This is out of order here. It means the tag name is not "template", "body", "html", or "br".
                Token::EndTag(tag)
                        if !["body", "html", "br"].contains(&tag.name.as_str()) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "An end tag whose tag name is one of: "body", "html", "br""
                // "Anything else"
                token => {
                    // "Insert an HTML element for a "body" start tag token with no attributes."
                    let tag = Tag {
                        name: InternedString::from_in("body", self.allocator),
                        ..Tag::new(self.allocator)
                    };
                    self.insert_html_element(document, tag);

                    // "Switch the insertion mode to "in body"."
                    self.insertion_mode = InsertionMode::InBody;

                    // "Reprocess the current token."
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inbody
            InsertionMode::InBody => match token {
                // "A character token that is U+0000 NULL"
                Token::Character('\0') => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // TODO: "Reconstruct the active formatting elements, if any."

                    // "Insert the token's character."
                    self.insert_character(c);
                },

                // "Any other character token"
                Token::Character(c) => {
                    // TODO: "Reconstruct the active formatting elements, if any."

                    // "Insert the token's character."
                    self.insert_character(c);

                    // "Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment."
                    let (parent, index) = self.appropriate_place_for_inserting_a_node(
                        self.current_node().expect("no current node")
                    );
                    Self::insert_comment(comment, parent.borrow_mut().children_mut(), index);
                },

                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(mut tag) if tag.name == "html" => {
                    // TODO: "Parse error"

                    // "If there is a template element on the stack of open elements, then ignore the token."
                    if self.open_elements.iter().any(|elem|
                                elem.borrow().as_elem().identifier == Identifier::new_html(
                                    InternedString::from_in("template", self.allocator)
                                )
                            ) {
                        return;
                    }

                    // "Otherwise, for each attribute on the token, check to see if the attribute is already present
                    // on the top element of the stack of open elements. If it is not, add the attribute and its
                    // corresponding value to that element."
                    let mut root_element = self.open_elements.first_mut().expect("no open elements").borrow_mut();
                    for tag_attr in tag.attributes.drain(..) {
                        if !root_element.as_elem().attributes.iter()
                                .any(|elem_attr| elem_attr.name == tag_attr.name) {
                            root_element.as_elem_mut().attributes.push(tag_attr);
                        }
                    }
                },

                // "A start tag whose tag name is one of: "base", "basefont", "bgsound", "link", "meta", "noframes",
                // "script", "style", "template", "title""
                Token::StartTag(ref tag)
                        if ["base", "basefont", "bgsound", "link", "meta", "noframes", "script",
                            "style", "template", "title"].contains(&tag.name.as_str()) => {
                    // "Process the token using the rules for the "in head" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InHead, document);
                },

                // "An end tag whose tag name is "template""
                Token::EndTag(ref tag) if tag.name == "template" => {
                    // "Process the token using the rules for the "in head" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InHead, document);
                },

                // "A start tag whose tag name is "body""
                Token::StartTag(mut tag) if tag.name == "body" => {
                    // TODO: "Parse error"

                    // "If the second element on the stack of open elements is not a body element, if the stack of
                    // open elements has only one node on it, or if there is a template element on the stack of open
                    // elements, then ignore the token. (fragment case)"
                    if self.open_elements.len() < 2 ||
                            self.open_elements[1].borrow().as_elem().identifier != Identifier::new_html(
                                InternedString::from_in("body", self.allocator)
                            ) ||
                            self.open_elements.iter().any(|elem|
                                elem.borrow().as_elem().identifier == Identifier::new_html(InternedString::from_in(
                                    "template", self.allocator
                                ))
                            ) {
                        return;
                    }

                    // "Otherwise, set the frameset-ok flag to "not ok"; then, for each attribute on the token, check
                    // to see if the attribute is already present on the body element (the second element) on the
                    // stack of open elements, and if it is not, add the attribute and its corresponding value to that
                    // element."
                    self.frameset_ok = false;
                    let body_element = &mut self.open_elements[1].borrow_mut();
                    for tag_attr in tag.attributes.drain(..) {
                        if !body_element.as_elem().attributes.iter()
                                .any(|elem_attr| elem_attr.name == tag_attr.name) {
                            body_element.as_elem_mut().attributes.push(tag_attr);
                        }
                    }
                },

                // "A start tag whose tag name is "frameset""
                Token::StartTag(tag) if tag.name == "frameset" => {
                    // TODO: "Parse error"

                    // "If the stack of open elements has only one node on it, or if the second element on the stack
                    // of open elements is not a body element, then ignore the token. (fragment case)"
                    // "If the frameset-ok flag is set to "not ok", ignore the token."
                    if !self.frameset_ok || self.open_elements.len() < 2 ||
                            self.open_elements[1].borrow().as_elem().identifier != Identifier::new_html(
                                InternedString::from_in("body", self.allocator)
                    ) {
                        return;
                    }

                    // "Otherwise, run the following steps:"
                    // "1. Remove the second element on the stack of open elements from its parent node, if it has one."
                    let element = &self.open_elements[1];
                    if let Some(parent) = element.borrow_mut().parent_mut().upgrade() {
                        parent.borrow_mut().remove_child(element);
                    }

                    // "2. Pop all the nodes from the bottom of the stack of open elements, from the current node up to,
                    // but not including, the root html element."
                    self.open_elements.truncate(1);

                    // "3. Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "4. Switch the insertion mode to "in frameset"."
                    self.insertion_mode = InsertionMode::InFrameset;
                },

                // "An end-of-file token"
                Token::EndOfFile => {
                    // "If the stack of template insertion modes is not empty, then process the token using the rules
                    // for the "in template" insertion mode."
                    if !self.template_insertion_modes.is_empty() {
                        return self.parse_token(token, tokenizer, InsertionMode::InTemplate, document);
                    }

                    // "Otherwise, follow these steps:"
                    // "1. If there is a node in the stack of open elements that is not either a dd element, a dt
                    // element, an li element, an optgroup element, an option element, a p element, an rb element, an rp
                    // element, an rt element, an rtc element, a tbody element, a td element, a tfoot element, a th
                    // element, a thead element, a tr element, the body element, or the html element, then this is a
                    // parse error."
                    assert!(!self.open_elements.is_empty());
                    if !self.open_elements.iter().any(|elem|
                            ["dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc", "tbody", "td",
                                "tfoot", "th", "thead", "tr", "body", "html"].iter().any(|tag_name|
                                    elem.borrow().as_elem().identifier == Identifier::new_html(
                                        InternedString::from_in(tag_name, self.allocator)
                                    )
                                )
                            ) {
                        // TODO: Parse error
                    }

                    // "2. Stop parsing."
                    self.stop_parsing();
                },

                // "An end tag whose tag name is "body""
                Token::EndTag(tag) if tag.name == "body" => {
                    // "If the stack of open elements does not have a body element in scope, this is a parse error;
                    // ignore the token."
                    if !self.has_element_in_scope("body") {
                        // TODO: Parse error
                        return;
                    }

                    // "Otherwise, if there is a node in the stack of open elements that is not either a dd element,
                    // a dt element, an li element, an optgroup element, an option element, a p element, an rb element,
                    // an rp element, an rt element, an rtc element, a tbody element, a td element, a tfoot element, a
                    // th element, a thead element, a tr element, the body element, or the html element, then this is a
                    // parse error."
                    assert!(!self.open_elements.is_empty());
                    if !self.open_elements.iter().any(|elem|
                            ["dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc", "tbody", "td",
                                "tfoot", "th", "thead", "tr", "body", "html"].iter().any(|tag_name| {
                                    let mut s = InternedString::new_in(self.allocator);
                                    s += tag_name;
                                    elem.borrow().as_elem().identifier == Identifier::new_html(s)
                                })
                            ) {
                        // TODO: Parse error
                    }

                    // "Switch the insertion mode to "after body"."
                    self.insertion_mode = InsertionMode::AfterBody;
                },

                // "An end tag whose tag name is "html""
                Token::EndTag(ref tag) if tag.name == "html" => {
                    // "If the stack of open elements does not have a body element in scope, this is a parse error;
                    // ignore the token."
                    if !self.has_element_in_scope("body") {
                        // TODO: Parse error
                        return;
                    }

                    // "Otherwise, if there is a node in the stack of open elements that is not either a dd element,
                    // a dt element, an li element, an optgroup element, an option element, a p element, an rb element,
                    // an rp element, an rt element, an rtc element, a tbody element, a td element, a tfoot element, a
                    // th element, a thead element, a tr element, the body element, or the html element, then this is a
                    // parse error."
                    assert!(!self.open_elements.is_empty());
                    if !self.open_elements.iter().any(|elem|
                            ["dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc", "tbody", "td",
                                "tfoot", "th", "thead", "tr", "body", "html"].iter().any(|tag_name| {
                                    let mut s = InternedString::new_in(self.allocator);
                                    s += tag_name;
                                    elem.borrow().as_elem().identifier == Identifier::new_html(s)
                                })
                            ) {
                        // TODO: Parse error
                    }

                    // "Switch the insertion mode to "after body"."
                    self.insertion_mode = InsertionMode::AfterBody;

                    // "Reprocess the token."
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                },

                // "A start tag whose tag name is one of: "address", "article", "aside", "blockquote", "center",
                // "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure", "footer", "header",
                // "hgroup", "main", "menu", "nav", "ol", "p", "section", "summary", "ul""
                Token::StartTag(tag)
                        if ["address", "article", "aside", "blockquote", "center", "details", "dialog", "dir", "div",
                            "dl", "fieldset", "figcaption", "figure", "footer", "header", "hgroup", "main", "menu",
                            "nav", "ol", "p", "section", "summary", "ul"].contains(&tag.name.as_str()) => {
                    // "If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);
                },

                // "A start tag whose tag name is one of: "h1", "h2", "h3", "h4", "h5", "h6""
                Token::StartTag(tag)
                        if ["h1", "h2", "h3", "h4", "h5", "h6"].contains(&tag.name.as_str()) => {
                    // "If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // "If the current node is an HTML element whose tag name is one of "h1", "h2", "h3", "h4", "h5",
                    // or "h6", then this is a parse error; pop the current node off the stack of open elements."
                    let elem = self.current_node().expect("no open elements");
                    if ["h1", "h2", "h3", "h4", "h5", "h6"].iter().any(|tag_name| {
                            let mut s = InternedString::new_in(self.allocator);
                            s += tag_name;
                            elem.borrow().as_elem().identifier == Identifier::new_html(s)
                    }) {
                        // TODO: Parse error
                        self.open_elements.pop().expect("no open elements");
                    }

                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);
                },

                // "A start tag whose tag name is one of: "pre", "listing""
                Token::StartTag(tag) if ["pre", "listing"].contains(&tag.name.as_str()) => {
                    // "If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "If the next token is a U+000A LINE FEED (LF) character token, then ignore that token and move
                    // on to the next one. (Newlines at the start of pre blocks are ignored as an authoring
                    // convenience.)"
                    self.skip_newline = true;

                    // "Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;
                },

                // "A start tag whose tag name is "form""
                Token::StartTag(tag) if tag.name == "form" => {
                    // TODO: "If the form element pointer is not null, and there is no template element on the stack
                    // of open elements, then this is a parse error; ignore the token."

                    // "Otherwise:"
                    // "If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // TODO: "Insert an HTML element for the token, and, if there is no template element on the
                    // stack of open elements, set the form element pointer to point to the element created."
                    self.insert_html_element(document, tag);
                    todo!();
                },

                // "A start tag whose tag name is "li""
                Token::StartTag(tag) if tag.name == "li" => {
                    let li_id = Identifier::new_html(InternedString::from_in("li", self.allocator));

                    // "1. Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;

                    // "2. Initialize node to be the current node (the bottommost node of the stack)."
                    for node in self.open_elements.iter().rev() {
                        // "3. Loop: If node is an li element, then run these substeps:"
                        if node.borrow().as_elem().identifier == li_id {
                            // "1. Generate implied end tags, except for li elements."
                            self.generate_implied_end_tags_except(&["li"]);

                            // "2. If the current node is not an li element, then this is a parse error."
                            match self.current_node() {
                                Some(ref node) => match *node.borrow() {
                                    Node::Element(ref elem) if elem.identifier == li_id => {},
                                    _ => {
                                        // TODO: Parse error
                                    }
                                },
                                _ => {
                                    // TODO: Parse error
                                }
                            };

                            // "3. Pop elements from the stack of open elements until an li element has been
                            // popped from the stack."
                            loop {
                                let node = self.open_elements.pop().expect("no open li element");
                                if node.borrow().as_elem().identifier == li_id {
                                    break;
                                }
                            }

                            // "4. Jump to the step labeled done below."
                            break;
                        }

                        // "4. If node is in the special category, but is not an address, div, or p element, then
                        // jump to the step labeled done below."
                        let node = node.borrow();
                        let elem = node.as_elem();
                        if elem.is_special() && !(
                                    elem.identifier.namespace == namespace::HTML &&
                                    ["address", "div", "p"].contains(&elem.identifier.local_name.as_str())
                                ) {
                            break;
                        }

                        // "5. Otherwise, set node to the previous entry in the stack of open elements and return to the
                        // step labeled loop."
                        // NOTE: This is done implicitly by the `for` loop.
                    }

                    // "6. Done: If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // "7. Finally, insert an HTML element for the token."
                    self.insert_html_element(document, tag);
                },

                // "A start tag whose name is one of: "dd", "dt""
                Token::StartTag(tag) if ["dd", "dt"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "plaintext""
                Token::StartTag(tag) if tag.name == "plaintext" => {
                    // "If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "Switch the tokenizer to the PLAINTEXT state."
                    tokenizer.state = TokenizerState::Plaintext;
                },

                // "A start tag whose tag name is "button""
                Token::StartTag(tag) if tag.name == "button" => {
                    // "1. If the stack of open elements has a button element in scope, then run these substeps:"
                    if self.has_element_in_scope("button") {
                        // TODO: "1. Parse error."

                        // "2. Generate implied end tags."
                        self.generate_implied_end_tags();

                        // "3. Pop elements from the stack of open elements until a button element has been popped from
                        // the stack."
                        let button_id = Identifier::new_html(InternedString::from_in("button", self.allocator));
                        while self.open_elements.pop().expect("no current node").borrow().as_elem().identifier
                                != button_id {}
                    }

                    // TODO: "2. Reconstruct the active formatting elements, if any."

                    // "3. Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "4. Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;
                },

                // "An end tag whose tag name is one of: "address", "article", "aside", "blockquote", "button",
                // "center", "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure", "footer",
                // "header", "hgroup", "listing", "main", "menu", "nav", "ol", "pre", "section", "summary", "ul""
                Token::EndTag(tag)
                        if ["address", "article", "aside", "blockquote", "button", "center", "details", "dialog", "dir",
                            "div", "dl", "fieldset", "figcaption", "figure", "footer", "header", "hgroup", "listing",
                            "main", "menu", "nav", "ol", "pre", "section", "summary", "ul"]
                            .contains(&tag.name.as_str()) => {
                    // "If the stack of open elements does not have an element in scope that is an HTML element with
                    // the same tag name as that of the token, then this is a parse error; ignore the token."
                    if !self.has_element_in_scope(tag.name.as_str()) {
                        // TODO: Parse error
                        return;
                    }

                    // "Otherwise, run these steps:"
                    // "1. Generate implied end tags."
                    self.generate_implied_end_tags();

                    // "2. If the current node is not an HTML element with the same tag name as that of the token, then
                    // this is a parse error."
                    let tag_id = Identifier::new_html(tag.name);
                    if self.current_node().expect("no current node").borrow().as_elem().identifier != tag_id {
                        // TODO: Parse error
                    }

                    // "3. Pop elements from the stack of open elements until an HTML element with the same tag name as
                    // the token has been popped from the stack."
                    while self.open_elements.pop().expect("no current node").borrow().as_elem().identifier != tag_id {}
                },

                // "An end tag whose tag name is "form""
                Token::EndTag(tag) if tag.name == "form" => {
                    // TODO
                    todo!()
                },

                // "An end tag whose tag name is "p""
                Token::EndTag(tag) if tag.name == "p" => {
                    // "If the stack of open elements does not have a p element in button scope, then this is a parse
                    // error; insert an HTML element for a "p" start tag token with no attributes."
                    if !self.has_element_in_button_scope("p") {
                        // TODO: Parse error
                        self.insert_html_element(document, Tag {
                            name: tag.name,
                            self_closing: false,
                            attributes: Vec::new_in(self.allocator)
                        });
                    }

                    // "Close a p element."
                    self.close_a_p_element();
                },

                // "An end tag whose tag name is "li""
                Token::EndTag(tag) if tag.name == "li" => {
                    // "If the stack of open elements does not have an li element in list item scope, then this is a
                    // parse error; ignore the token."
                    if !self.has_element_in_list_item_scope("li") {
                        // TODO: Parse error
                        return;
                    }

                    // "Otherwise, run these steps:"
                    // "1. Generate implied end tags, except for li elements."
                    self.generate_implied_end_tags_except(&["li"]);

                    // "2. If the current node is not an li element, then this is a parse error."
                    let li_id = Identifier::new_html(InternedString::from_in("li", self.allocator));
                    match *self.current_node().expect("no current node").borrow() {
                        Node::Element(ref elem) if elem.identifier == li_id => {},
                        _ => {
                            // TODO: Parse error
                        }
                    };

                    // "Pop elements from the stack of open elements until an li element has been popped from the
                    // stack."
                    while self.open_elements.pop().expect("no current node").borrow().as_elem().identifier != li_id {}
                },

                // "An end tag whose tag name is one of: "dd", "dt""
                Token::EndTag(tag) if ["dd", "dt"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "An end tag whose tag name is one of: "h1", "h2", "h3", "h4", "h5", "h6""
                Token::EndTag(tag) if ["h1", "h2", "h3", "h4", "h5", "h6"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "a""
                Token::StartTag(tag) if tag.name == "a" => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is one of: "b", "big", "code", "em", "font", "i", "s", "small",
                // "strike", "strong", "tt", "u""
                Token::StartTag(tag)
                        if ["b", "big", "code", "em", "font", "i", "s", "small", "strike", "strong", "tt", "u"]
                            .contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "nobr""
                Token::StartTag(tag) if tag.name == "nobr" => {
                    // TODO
                    todo!()
                },

                // "An end tag whose tag name is one of: "a", "b", "big", "code", "em", "font", "i", "nobr", "s",
                // "small", "strike", "strong", "tt", "u""
                Token::EndTag(ref tag)
                        if ["a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt",
                            "u"].contains(&tag.name.as_str()) => {
                    // TODO: "Run the adoption agency algorithm for the token."
                    todo!()
                },

                // "A start tag whose tag name is one of: "applet", "marquee", "object""
                Token::StartTag(tag) if ["applet", "marquee", "object"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "An end tag token whose tag name is one of: "applet", "marquee", "object""
                Token::EndTag(tag) if ["applet", "marquee", "object"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "table""
                Token::StartTag(tag) if tag.name == "table" => {
                    // TODO
                    todo!()
                },

                // "An end tag whose tag name is "br""
                Token::EndTag(mut tag) if tag.name == "br" => {
                    // "Parse error. Drop the attributes from the token, and act as described in the next entry; i.e.
                    // act as if this was a "br" start tag token with no attributes, rather than the end tag token that
                    // it actually is."
                    // TODO: Parse error
                    tag.attributes.clear();
                    self.parse_token(Token::StartTag(tag), tokenizer, self.insertion_mode, document);
                },

                // "A start tag whose tag name is one of: "area", "br", "embed", "img", "keygen", "wbr""
                Token::StartTag(tag) if ["area", "br", "embed", "img", "keygen", "wbr"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "input""
                Token::StartTag(tag) if tag.name == "input" => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is one of: "param", "source", "track""
                Token::StartTag(tag) if ["param", "source", "track"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "hr""
                Token::StartTag(tag) if tag.name == "hr" => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "image""
                Token::StartTag(ref mut tag) if tag.name == "image" => {
                    // "Parse error. Change the token's tag name to "img" and reprocess it. (Don't ask.)"
                    // TODO: Parse error
                    tag.name = InternedString::from_in("img", self.allocator);
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                },

                // "A start tag whose tag name is "textarea""
                Token::StartTag(tag) if tag.name == "textarea" => {
                    // "1. Insert an HTML element for the token."
                    self.insert_html_element(document, tag);

                    // "2. If the next token is a U+000A LINE FEED (LF) character token, then ignore that token and
                    // move on to the next one. (Newlines at the start of textarea elements are ignored as an
                    // authoring convenience.)"
                    self.skip_newline = true;

                    // "3. Switch the tokenizer to the RCDATA state."
                    tokenizer.state = TokenizerState::Rcdata;

                    // "4. Let the original insertion mode be the current insertion mode."
                    self.original_insertion_mode = Some(self.insertion_mode);

                    // "5. Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;

                    // "6. Switch the insertion mode to "text"."
                    self.insertion_mode = InsertionMode::Text;
                },

                // "A start tag whose tag name is "xmp""
                Token::StartTag(tag) if tag.name == "xmp" => {
                    // "If the stack of open elements has a p element in button scope, then close a p element."
                    if self.has_element_in_button_scope("p") {
                        self.close_a_p_element();
                    }

                    // TODO: "Reconstruct the active formatting elements, if any."

                    // "Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;

                    // "Follow the generic raw text element parsing algorithm."
                    self.parse_text_element(tokenizer, TokenizerState::Rawtext, document, tag);
                },

                // "A start tag whose tag name is "iframe""
                Token::StartTag(tag) if tag.name == "iframe" => {
                    // "Set the frameset-ok flag to "not ok"."
                    self.frameset_ok = false;

                    // "Follow the generic raw text element parsing algorithm."
                    self.parse_text_element(tokenizer, TokenizerState::Rawtext, document, tag);
                },

                // "A start tag whose tag name is "noembed""
                // "A start tag whose tag name is "noscript", if the scripting flag is enabled"
                Token::StartTag(tag) if tag.name == "noembed" || (tag.name == "noscript" && self.scripting) => {
                    // "Follow the generic raw text element parsing algorithm."
                    self.parse_text_element(tokenizer, TokenizerState::Rawtext, document, tag);
                },

                // "A start tag whose tag name is "select""
                Token::StartTag(tag) if tag.name == "select" => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is one of: "optgroup", "option""
                Token::StartTag(tag) if ["optgroup", "option"].contains(&tag.name.as_str()) => {
                    // "If the current node is an option element, then pop the current node off the stack of open
                    // elements."
                    if self.current_node().expect("no current node").borrow().as_elem().identifier ==
                            Identifier::new_html(InternedString::from_in("option", self.allocator)) {
                        self.open_elements.pop().expect("no current node");
                    }

                    // TODO: "Reconstruct the active formatting elements, if any."

                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);
                },

                // "A start tag whose tag name is one of: "rb", "rtc""
                Token::StartTag(tag) if ["rb", "rtc"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is one of: "rp", "rt""
                Token::StartTag(tag) if ["rp", "rt"].contains(&tag.name.as_str()) => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "math""
                Token::StartTag(tag) if tag.name == "math" => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is "svg""
                Token::StartTag(tag) if tag.name == "svg" => {
                    // TODO
                    todo!()
                },

                // "A start tag whose tag name is one of: "caption", "col", "colgroup", "frame", "head", "tbody", "td",
                // "tfoot", "th", "thead", "tr""
                Token::StartTag(tag)
                        if ["caption", "col", "colgroup", "frame", "head", "tbody", "td", "tfoot", "th", "thead", "tr"]
                            .contains(&tag.name.as_str()) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "Any other start tag"
                Token::StartTag(tag) => {
                    // TODO: "Reconstruct the active formatting elements, if any."

                    // "Insert an HTML element for the token."
                    self.insert_html_element(document, tag);
                },

                // "An end tag whose tag name is "sarcasm""
                // "Any other end tag"
                Token::EndTag(tag) => {
                    let tag_id = Identifier::new_html(tag.name.clone());

                    // "1. Initialize node to be the current node (the bottommost node of the stack)."
                    for i in (0 .. self.open_elements.len()).rev() {
                        let node = &self.open_elements[i];

                        // "2. Loop: If node is an HTML element with the same tag name as the token, then:"
                        if node.borrow().as_elem().identifier == tag_id {
                            // "1. Generate implied end tags, except for HTML elements with the same tag name as the
                            // token."
                            self.generate_implied_end_tags_except(&[tag.name.as_str()]);

                            // "2. If node is not the current node, then this is a parse error."
                            if self.open_elements.len() != i + 1 {
                                // TODO: Parse error
                            }

                            // "3. Pop all the nodes from the current node up to node, including node, then stop these
                            // steps."
                            self.open_elements.truncate(i);
                            break;
                        }

                        // "3. Otherwise, if node is in the special category, then this is a parse error; ignore the
                        // token, and return."
                        if node.borrow().as_elem().is_special() {
                            // TODO: Parse error
                            return;
                        }

                        // "4. Set node to the previous entry in the stack of open elements."
                        // "5. Return to the step labeled loop."
                        // NOTE: Both of these steps are done implicitly by the `for` loop.
                    }
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incdata
            InsertionMode::Text => match token {
                // "A character token"
                Token::Character(c) => {
                    // "Insert the token's character."
                    self.insert_character(c);
                },

                // "An end-of-file token"
                Token::EndOfFile => {
                    // TODO: "Parse error."

                    // TODO: "If the current node is a script element, mark the script element as "already started"."

                    // "Pop the current node off the stack of open elements."
                    self.open_elements.pop().expect("no current node");

                    // "Switch the insertion mode to the original insertion mode and reprocess the token."
                    self.insertion_mode = self.original_insertion_mode.expect("no original insertion mode");
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                },

                // "An end tag whose tag name is "script""
                Token::EndTag(ref tag) if tag.name == "script" => {
                    // TODO
                    todo!()
                },

                // "Any other end tag"
                Token::EndTag(tag) => {
                    // "Pop the current node off the stack of open elements."
                    self.open_elements.pop().expect("no current node");

                    // "Switch the insertion mode to the original insertion mode."
                    self.insertion_mode = self.original_insertion_mode.expect("no original insertion mode");
                },
                
                _ => {
                    panic!("unexpected token in text insertion mode")
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-intable
            InsertionMode::InTable => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-intabletext
            InsertionMode::InTableText => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incaption
            InsertionMode::InCaption => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-incolgroup
            InsertionMode::InColumnGroup => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-intbody
            InsertionMode::InTableBody => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-intr
            InsertionMode::InRow => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-intd
            InsertionMode::InCell => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inselect
            InsertionMode::InSelect => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inselectintable
            InsertionMode::InSelectInTable => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-intemplate
            InsertionMode::InTemplate => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterbody
            InsertionMode::AfterBody => match token {
                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment as the last child of the first element of the stack of open elements
                    // (the html element)."
                    let parent = &self.open_elements[0];
                    let index = parent.borrow().children().len();
                    Self::insert_comment(comment, parent.borrow_mut().children_mut(), index);
                },

                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Parse error. Ignore the token."
                    // TODO: Parse error
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(ref tag) if tag.name == "html" => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "An end tag whose tag name is "html""
                Token::EndTag(tag) if tag.name == "html" => {
                    // "If the parser was created as part of the HTML fragment parsing algorithm, this is a parse
                    // error; ignore the token. (fragment case)"
                    if self.html_fragment_parsing_algorithm_context.is_some() {
                        // TODO: Parse error
                        return;
                    }

                    // "Otherwise, switch the insertion mode to "after after body"."
                    self.insertion_mode = InsertionMode::AfterAfterBody;
                },

                // "An end-of-file token"
                Token::EndOfFile => {
                    // "Stop parsing."
                    self.stop_parsing();
                },

                // "Anything else"
                token => {
                    // "Parse error. Switch the insertion mode to "in body" and reprocess the token."
                    // TODO: Parse error
                    self.insertion_mode = InsertionMode::InBody;
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-inframeset
            InsertionMode::InFrameset => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#parsing-main-afterframeset
            InsertionMode::AfterFrameset => todo!(),

            // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-body-insertion-mode
            InsertionMode::AfterAfterBody => match token {
                // "A comment token"
                Token::Comment(comment) => {
                    // "Insert a comment as the last child of the Document object."
                    let index = document.dom.children.len();
                    Self::insert_comment(comment, &mut document.dom.children, index);
                },

                // "A DOCTYPE token"
                Token::Doctype(_) => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "A character token that is one of U+0009 CHARACTER TABULATION, U+000A LINE FEED (LF), U+000C FORM
                // FEED (FF), U+000D CARRIAGE RETURN (CR), or U+0020 SPACE"
                Token::Character(c) if c.is_ascii_whitespace() => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "A start tag whose tag name is "html""
                Token::StartTag(ref tag) if tag.name == "html" => {
                    // "Process the token using the rules for the "in body" insertion mode."
                    self.parse_token(token, tokenizer, InsertionMode::InBody, document);
                },

                // "An end-of-file token"
                Token::EndOfFile => {
                    // "Stop parsing."
                    self.stop_parsing();
                },

                // "Anything else"
                token => {
                    // "Parse error. Switch the insertion mode to "in body" and reprocess the token."
                    // TODO: Parse error
                    self.insertion_mode = InsertionMode::InBody;
                    self.parse_token(token, tokenizer, self.insertion_mode, document);
                }
            },

            // https://html.spec.whatwg.org/multipage/parsing.html#the-after-after-frameset-insertion-mode
            InsertionMode::AfterAfterFrameset => todo!()
        };
    }

    fn get_quirks_mode_from_public_identifier(
            &self,
            public_identifier: &String<A>,
            system_identifier_exists: bool
    ) -> QuirksMode {
        match crate::shim::to_ascii_lowercase(public_identifier.as_str(), self.allocator).as_str() {
            "-//w3o//dtd w3 html strict 3.0//en//" |
            "-/w3c/dtd html 4.0 transitional/en" |
            "html" => QuirksMode::Quirks,

            id if id.starts_with("+//silmaril//dtd html pro v0r11 19970101//") ||
                id.starts_with("-//as//dtd html 3.0 aswedit + extensions//") ||
                id.starts_with("-//advasoft ltd//dtd html 3.0 aswedit + extensions//") ||
                id.starts_with("-//ietf//dtd html 2.0 level 1//") ||
                id.starts_with("-//ietf//dtd html 2.0 level 2//") ||
                id.starts_with("-//ietf//dtd html 2.0 strict level 1//") ||
                id.starts_with("-//ietf//dtd html 2.0 strict level 2//") ||
                id.starts_with("-//ietf//dtd html 2.0 strict//") ||
                id.starts_with("-//ietf//dtd html 2.0//") ||
                id.starts_with("-//ietf//dtd html 2.1e//") ||
                id.starts_with("-//ietf//dtd html 3.0//") ||
                id.starts_with("-//ietf//dtd html 3.2 final//") ||
                id.starts_with("-//ietf//dtd html 3.2//") ||
                id.starts_with("-//ietf//dtd html 3//") ||
                id.starts_with("-//ietf//dtd html level 0//") ||
                id.starts_with("-//ietf//dtd html level 1//") ||
                id.starts_with("-//ietf//dtd html level 2//") ||
                id.starts_with("-//ietf//dtd html level 3//") ||
                id.starts_with("-//ietf//dtd html strict level 0//") ||
                id.starts_with("-//ietf//dtd html strict level 1//") ||
                id.starts_with("-//ietf//dtd html strict level 2//") ||
                id.starts_with("-//ietf//dtd html strict level 3//") ||
                id.starts_with("-//ietf//dtd html strict//") ||
                id.starts_with("-//ietf//dtd html//") ||
                id.starts_with("-//metrius//dtd metrius presentational//") ||
                id.starts_with("-//microsoft//dtd internet explorer 2.0 html strict//") ||
                id.starts_with("-//microsoft//dtd internet explorer 2.0 html//") ||
                id.starts_with("-//microsoft//dtd internet explorer 2.0 tables//") ||
                id.starts_with("-//microsoft//dtd internet explorer 3.0 html strict//") ||
                id.starts_with("-//microsoft//dtd internet explorer 3.0 html//") ||
                id.starts_with("-//microsoft//dtd internet explorer 3.0 tables//") ||
                id.starts_with("-//netscape comm. corp.//dtd html//") ||
                id.starts_with("-//netscape comm. corp.//dtd strict html//") ||
                id.starts_with("-//o'reilly and associates//dtd html 2.0//") ||
                id.starts_with("-//o'reilly and associates//dtd html extended 1.0//") ||
                id.starts_with("-//o'reilly and associates//dtd html extended relaxed 1.0//") ||
                id.starts_with("-//sq//dtd html 2.0 hotmetal + extensions//") ||
                id.starts_with("-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//") ||
                id.starts_with("-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//") ||
                id.starts_with("-//spyglass//dtd html 2.0 extended//") ||
                id.starts_with("-//sun microsystems corp.//dtd hotjava html//") ||
                id.starts_with("-//sun microsystems corp.//dtd hotjava strict html//") ||
                id.starts_with("-//w3c//dtd html 3 1995-03-24//") ||
                id.starts_with("-//w3c//dtd html 3.2 draft//") ||
                id.starts_with("-//w3c//dtd html 3.2 final//") ||
                id.starts_with("-//w3c//dtd html 3.2//") ||
                id.starts_with("-//w3c//dtd html 3.2s draft//") ||
                id.starts_with("-//w3c//dtd html 4.0 frameset//") ||
                id.starts_with("-//w3c//dtd html 4.0 transitional//") ||
                id.starts_with("-//w3c//dtd html experimental 19960712//") ||
                id.starts_with("-//w3c//dtd html experimental 970421//") ||
                id.starts_with("-//w3c//dtd w3 html//") ||
                id.starts_with("-//w3o//dtd w3 html 3.0//") ||
                id.starts_with("-//webtechs//dtd mozilla html 2.0//") ||
                id.starts_with("-//webtechs//dtd mozilla html//") ||
                (
                    !system_identifier_exists &&
                    (
                        id.starts_with("-//w3c//dtd html 4.01 frameset//") ||
                        id.starts_with("-//w3c//dtd html 4.01 transitional//")
                    )
                ) => QuirksMode::Quirks,

            id if id.starts_with("-//w3c//dtd xhtml 1.0 frameset//") ||
                id.starts_with("-//w3c//dtd xhtml 1.0 transitional//") ||
                (
                    system_identifier_exists &&
                    (
                        id.starts_with("-//w3c//dtd html 4.01 frameset//") ||
                        id.starts_with("-//w3c//dtd html 4.01 transitional//")
                    )
                ) => QuirksMode::LimitedQuirks,

            _ => QuirksMode::NoQuirks
        }
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#change-the-encoding
    fn change_encoding(mut new_encoding: CharEncoding, tokenizer: &Tokenizer<A>, document: &mut DocumentInternal<A>) {
        let old_encoding = *tokenizer.encoding.borrow();
        if old_encoding == CharEncoding::Utf16Be || old_encoding == CharEncoding::Utf16Le {
            document.enc_confidence = CharEncodingConfidence::Certain;
            return;
        }

        if new_encoding == CharEncoding::Utf16Be || new_encoding == CharEncoding::Utf16Le {
            new_encoding = CharEncoding::Utf8;
        } else if new_encoding == CharEncoding::XUserDefined {
            new_encoding = CharEncoding::Windows1252;
        }

        if new_encoding == old_encoding {
            document.enc_confidence = CharEncodingConfidence::Certain;
            return;
        }

        // TODO: We should actually change the encoding here.
        todo!()
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#current-node
    fn current_node(&self) -> Option<&Rc<RefCell<Node<A>>>> {
        self.open_elements.last()
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#adjusted-current-node
    fn adjusted_current_node(&self) -> Option<&Rc<RefCell<Node<A>>>> {
        match self.html_fragment_parsing_algorithm_context {
            Some(ref context) if self.open_elements.len() == 1 => Some(context),
            _ =>                                                  self.current_node()
        }
    }

    // "Have an element target node in a specific scope"
    fn has_element_in_specific_scope(&self, element: &str, scope: &[Identifier<A>]) -> bool {
        let mut open_elements = self.open_elements.iter().rev();
        let mut node = open_elements.next().expect("no current node");
        let elem_id = Identifier::new_html(InternedString::from_in(element, self.allocator));
        loop {
            if node.borrow().as_elem().identifier == elem_id {
                return true;
            }
            if scope.contains(&elem_id) {
                return false;
            }
            node = open_elements.next().expect("no next node");
        }
    }

    // "Have a particular element in scope"
    fn has_element_in_scope(&self, element: &str) -> bool {
        let scope = [
            Identifier::new_html(InternedString::from_in("applet", self.allocator)),
            Identifier::new_html(InternedString::from_in("caption", self.allocator)),
            Identifier::new_html(InternedString::from_in("html", self.allocator)),
            Identifier::new_html(InternedString::from_in("table", self.allocator)),
            Identifier::new_html(InternedString::from_in("td", self.allocator)),
            Identifier::new_html(InternedString::from_in("th", self.allocator)),
            Identifier::new_html(InternedString::from_in("marquee", self.allocator)),
            Identifier::new_html(InternedString::from_in("object", self.allocator)),
            Identifier::new_html(InternedString::from_in("template", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mi", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mo", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mn", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("ms", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mtext", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("annotation-xml", self.allocator)),
            Identifier::new_svg(InternedString::from_in("foreignObject", self.allocator)),
            Identifier::new_svg(InternedString::from_in("desc", self.allocator)),
            Identifier::new_svg(InternedString::from_in("title", self.allocator))
        ];
        self.has_element_in_specific_scope(element, &scope)
    }

    // "Have a particular element in button scope"
    fn has_element_in_button_scope(&self, element: &str) -> bool {
        let scope = [
            Identifier::new_html(InternedString::from_in("applet", self.allocator)),
            Identifier::new_html(InternedString::from_in("caption", self.allocator)),
            Identifier::new_html(InternedString::from_in("html", self.allocator)),
            Identifier::new_html(InternedString::from_in("table", self.allocator)),
            Identifier::new_html(InternedString::from_in("td", self.allocator)),
            Identifier::new_html(InternedString::from_in("th", self.allocator)),
            Identifier::new_html(InternedString::from_in("marquee", self.allocator)),
            Identifier::new_html(InternedString::from_in("object", self.allocator)),
            Identifier::new_html(InternedString::from_in("template", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mi", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mo", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mn", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("ms", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mtext", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("annotation-xml", self.allocator)),
            Identifier::new_svg(InternedString::from_in("foreignObject", self.allocator)),
            Identifier::new_svg(InternedString::from_in("desc", self.allocator)),
            Identifier::new_svg(InternedString::from_in("title", self.allocator)),
            Identifier::new_html(InternedString::from_in("button", self.allocator))
        ];
        self.has_element_in_specific_scope(element, &scope)
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#has-an-element-in-list-item-scope
    fn has_element_in_list_item_scope(&self, element: &str) -> bool {
        let scope = [
            Identifier::new_html(InternedString::from_in("applet", self.allocator)),
            Identifier::new_html(InternedString::from_in("caption", self.allocator)),
            Identifier::new_html(InternedString::from_in("html", self.allocator)),
            Identifier::new_html(InternedString::from_in("table", self.allocator)),
            Identifier::new_html(InternedString::from_in("td", self.allocator)),
            Identifier::new_html(InternedString::from_in("th", self.allocator)),
            Identifier::new_html(InternedString::from_in("marquee", self.allocator)),
            Identifier::new_html(InternedString::from_in("object", self.allocator)),
            Identifier::new_html(InternedString::from_in("template", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mi", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mo", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mn", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("ms", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("mtext", self.allocator)),
            Identifier::new_mathml(InternedString::from_in("annotation-xml", self.allocator)),
            Identifier::new_svg(InternedString::from_in("foreignObject", self.allocator)),
            Identifier::new_svg(InternedString::from_in("desc", self.allocator)),
            Identifier::new_svg(InternedString::from_in("title", self.allocator)),
            Identifier::new_html(InternedString::from_in("ol", self.allocator)),
            Identifier::new_html(InternedString::from_in("ul", self.allocator))
        ];
        self.has_element_in_specific_scope(element, &scope)
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#create-an-element-for-the-token
    fn element_from_tag(
            &self,
            document: &DocumentInternal<A>,
            mut tag: Tag<A>,
            namespace: &'static str,
            parent: Weak<RefCell<Node<A>>>
    ) -> Rc<RefCell<Node<A>>> {
        let is = tag.attributes.iter()
            .find(|attr| attr.name == InternedString::from_in("is", self.allocator))
            .map(|attr| InternedString::from_in(&attr.value, self.allocator));
        let definition = element::look_up_custom_element_definition(document, namespace, &tag.name, &is);
        let will_execute_script = definition.is_some() && self.html_fragment_parsing_algorithm_context.is_none();
        if will_execute_script {
            todo!();
        }
        let element = Element::new(document, tag.name, namespace, None, is, will_execute_script, self.allocator);
        for attribute in tag.attributes.drain(..) {
            Element::append_attribute(&element, attribute);
        }
        if will_execute_script {
            todo!();
        }

        // TODO: "If element has an xmlns attribute in the XMLNS namespace whose value is not exactly the same as the
        // element's namespace, that is a parse error. Similarly, if element has an xmlns:xlink attribute in the XMLNS
        // namespace whose value is not the XLink Namespace, that is a parse error."

        if element.borrow().as_elem().is_resettable() {
            todo!();
        }

        // TODO: "If element is a form-associated element and not a form-associated custom element, the form element
        // pointer is not null, there is no template element on the stack of open elements, element is either not
        // listed or doesn't have a form attribute, and the intended parent is in the same tree as the element pointed
        // to by the form element pointer, then associate element with the form element pointed to by the form element pointer and set element's parser inserted flag."

        element
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#generic-rcdata-element-parsing-algorithm
    fn parse_text_element(
            &mut self,
            tokenizer: &mut Tokenizer<A>,
            tokenizer_state: TokenizerState,
            document: &mut DocumentInternal<A>,
            tag: Tag<A>
    ) {
        // "1. Insert an HTML element for the token."
        self.insert_html_element(document, tag);

        // "2. If the algorithm that was invoked is the generic raw text element parsing algorithm, switch the
        // tokenizer to the RAWTEXT state; otherwise the algorithm invoked was the generic RCDATA element parsing
        // algorithm, switch the tokenizer to the RCDATA state."
        tokenizer.state = tokenizer_state;

        // "3. Let the original insertion mode be the current insertion mode."
        self.original_insertion_mode = Some(self.insertion_mode);

        // "4. Then, switch the insertion mode to "text"."
        self.insertion_mode = InsertionMode::Text;
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#insert-a-comment
    fn insert_comment(data: String<A>, siblings: &mut Vec<Rc<RefCell<Node<A>>>, A>, index: usize) {
        // "1. Let data be the data given in the comment token being processed."
        // NOTE: Implicit.

        // "2. If position was specified, then let the adjusted insertion location be position. Otherwise, let
        // adjusted insertion location be the appropriate place for inserting a node."
        // NOTE: Implicit.

        // "3. Create a Comment node whose data attribute is set to data and whose node document is the same as that
        // of the node in which the adjusted insertion location finds itself."
        let node = Rc::new(RefCell::new(Node::Comment(
            Comment { data }
        )));

        // "4. Insert the newly created node at the adjusted insertion location."
        siblings.insert(index, node);
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#the-initial-insertion-mode
    fn insert_doctype(&mut self, document: &mut DocumentInternal<A>, doctype: Doctype<A>) {
        // "Append a DocumentType node to the Document node, with its name set to the name given in the DOCTYPE token,
        // or the empty string if the name was missing; its public ID set to the public identifier given in the DOCTYPE
        // token, or the empty string if the public identifier was missing; and its system ID set to the system
        // identifier given in the DOCTYPE token, or the empty string if the system identifier was missing."
        let node = Rc::new(RefCell::new(Node::DocumentType(DocumentType::from_in(doctype, self.allocator))));
        document.dom.children.push(node.clone());
        document.document_type = Some(node);
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#insert-an-html-element
    fn insert_html_element(&mut self, document: &mut DocumentInternal<A>, tag: Tag<A>) -> &Rc<RefCell<Node<A>>> {
        self.insert_foreign_element(document, tag, namespace::HTML)
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#insert-a-foreign-element
    fn insert_foreign_element(
            &mut self,
            document: &mut DocumentInternal<A>,
            tag: Tag<A>,
            namespace: &'static str
    ) -> &Rc<RefCell<Node<A>>> {
        // "1. Let the adjusted insertion location be the appropriate place for inserting a node."
        let (parent, insertion_index) = self.appropriate_place_for_inserting_a_node(
            self.current_node().expect("no current node")
        );

        // "2. Let element be the result of creating an element for the token in the given namespace, with the
        // intended parent being the element in which the adjusted insertion location finds itself."
        let element = self.element_from_tag(document, tag, namespace, Rc::downgrade(parent));

        // "3. If it is possible to insert element at the adjusted insertion location, then:"
        if parent.borrow().can_insert_child(insertion_index) {
            // "1. If the parser was not created as part of the HTML fragment parsing algorithm, then push a new
            // element queue onto element's relevant agent's custom element reactions stack."
            if self.html_fragment_parsing_algorithm_context.is_none() {
                // TODO: "Push a new element queue onto element's relevant agent's custom element reactions stack."
            }

            // "2. Insert element at the adjusted insertion location."
            parent.borrow_mut().children_mut().insert(insertion_index, element.clone());

            // "3. If the parser was not created as part of the HTML fragment parsing algorithm, then pop the
            // element queue from element's relevant agent's custom element reactions stack, and invoke custom
            // element reactions in that queue."
            if self.html_fragment_parsing_algorithm_context.is_none() {
                // TODO: "Pop the element queue from element's relevant agent's custom element reactions stack,
                // and invoke custom element reactions in that queue."
            }
        }

        // "4. Push element onto the stack of open elements so that it is the new current node."
        self.open_elements.push(element);

        // "5. Return element."
        self.current_node().expect("no current node")
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#insert-a-character
    fn insert_character(&mut self, c: char) {
        if let Some(current_node) = self.current_node() {
            let (parent, insertion_index) = self.appropriate_place_for_inserting_a_node(current_node);
            if let Node::Document(_) = *parent.borrow() {
                // According to the spec, a Document node can't contain Text nodes.
                return;
            }
            if insertion_index > 0 {
                if let Node::Text(ref mut text) = *parent.borrow_mut().children()[insertion_index - 1].borrow_mut() {
                    text.push(c);
                    return;
                }
            }
            let text = String::from_char_in(c, self.allocator);
            parent.borrow_mut().children_mut().insert(insertion_index, Rc::new(RefCell::new(Node::Text(text))));
        } else {
            // According to the spec, a Document node can't contain Text nodes.
        }
    }

    // "Appropriate place for inserting a node"
    // Returns the parent of the appropriate place and the index into its list of children.
    fn appropriate_place_for_inserting_a_node<'a>(&self, target: &'a Rc<RefCell<Node<A>>>)
            -> (&'a Rc<RefCell<Node<A>>>, usize) {
        let (adjusted_parent, adjusted_index);
        if self.foster_parenting_enabled && ["table", "tbody", "tfoot", "thead", "tr"].iter().any(|tag_name|
                target.borrow().as_elem().identifier == Identifier::new_html(
                    InternedString::from_in(tag_name, self.allocator)
                )
        ) {
            // TODO: "If foster parenting is enabled and target is a table, tbody, tfoot, thead, or tr element"
            todo!();
        } else {
            adjusted_parent = target;
            adjusted_index = adjusted_parent.borrow().as_elem().children.len();
        }

        if adjusted_parent.borrow().as_elem().identifier == Identifier::new_html(
                InternedString::from_in("template", self.allocator)
        ) {
            // TODO: "If the adjusted insertion location is inside a template element, let it instead be inside the
            // template element's template contents, after its last child (if any)."
            // (In other words, set adjusted_parent to the template contents.)
            todo!();
        }

        (adjusted_parent, adjusted_index)
    }

    // "Generate implied end tags"
    fn generate_implied_end_tags_except(&mut self, exceptions: &[&str]) {
        static RELEVANT_TAGS: [&str; 10] = [
            "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc"
        ];

        loop {
            match self.current_node() {
                Some(element)
                    if RELEVANT_TAGS.iter().filter(|tag_name| !exceptions.contains(tag_name))
                            .any(|tag_name| element.borrow().as_elem().identifier == Identifier::new_html(
                                InternedString::from_in(tag_name, self.allocator)
                            )) => {
                    self.open_elements.pop().expect("no current node");
                },
                _ => break
            };
        }
    }

    fn generate_implied_end_tags(&mut self) {
        self.generate_implied_end_tags_except(&[]);
    }

    // "Generate all implied end tags thoroughly"
    fn generate_implied_end_tags_thoroughly(&mut self) {
        static RELEVANT_TAGS: [&str; 18] = [
            "caption", "colgroup", "dd", "dt", "li", "optgroup", "option", "p",
            "rb", "rp", "rt", "rtc", "tbody", "td", "tfoot", "th", "thead", "tr"
        ];

        loop {
            match self.current_node() {
                Some(element) if RELEVANT_TAGS.iter().any(|tag_name|
                        element.borrow().as_elem().identifier == Identifier::new_html(
                            InternedString::from_in(tag_name, self.allocator)
                        )) => {
                    self.open_elements.pop().expect("no current node");
                },
                _ => break
            };
        }
    }

    // "Close a p element"
    fn close_a_p_element(&mut self) {
        self.generate_implied_end_tags_except(&["p"]);
        let p_id = Identifier::new_html(InternedString::from_in("p", self.allocator));
        if self.current_node().expect("no current node").borrow().as_elem().identifier != p_id {
            // TODO: Parse error
        }
        while self.open_elements.pop().expect("no current node").borrow().as_elem().identifier != p_id {}
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#stop-parsing
    fn stop_parsing(&mut self) {
        // TODO: "Set the insertion point to undefined."
        // TODO: "Update the current document readiness to "interactive"."
        self.open_elements.clear();
        // TODO: "While the list of scripts that will execute when the document has finished parsing is not empty: ..."
        // TODO: "Queue a global task on the DOM manipulation task source given the Document's relevant global object
        // to run the following substeps: ..."
        // TODO: "Spin the event loop until the set of scripts that will execute as soon as possible and the list of
        // scripts that will execute in order as soon as possible are empty."
        // TODO: "Spin the event loop until there is nothing that delays the load event in the Document."
        // TODO: "Queue a global task on the DOM manipulation task source given the Document's relevant global object
        // to run the following steps: ..."
        // TODO: "If the Document's print when loaded flag is set, then run the printing steps."
        // TODO: "The Document is now ready for post-load tasks."
    }
}

#[derive(Debug)]
pub(super) enum ParseResult<T, E> {
    Ok(T),
    Later,
    Err(E)
}

impl<T, E> From<Result<T, E>> for ParseResult<T, E> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(t) => Self::Ok(t),
            Err(e) => Self::Err(e)
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    /// The parser encountered an invalid UTF-8 sequence.
    InvalidUtf8,
    /// The parser encountered a null character in an invalid position.
    UnexpectedNullCharacter,

    /// The stream ended with the first character of a new tag.
    // "eof-before-tag-name"
    EofBeforeTagName,
    /// The stream ended before the end of the DOCTYPE.
    // "eof-in-doctype"
    EofInDoctype,
    /// The DOCTYPE didn't include a name.
    // "missing-doctype-name"
    MissingDoctypeName
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset
}

#[derive(Debug)]
struct PopFrontIterator<'a, T, A: alloc::alloc::Allocator> {
    queue: &'a mut VecDeque<T, A>
}

impl<'a, T, A: alloc::alloc::Allocator> PopFrontIterator<'a, T, A> {
    fn new(queue: &'a mut VecDeque<T, A>) -> Self {
        Self { queue }
    }
}

impl<'a, T, A: alloc::alloc::Allocator> Iterator for PopFrontIterator<'a, T, A> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use {
        std::{
            io::Read,
            fs::File
        },
        super::{
            *,
            super::*
        }
    };

    macro_rules! file_test {
        (
            $test_name:ident,
            byte stream from $file_name:expr,
            $initial_document:expr,
            $closure:expr
        ) => {
            #[test]
            fn $test_name() {
                let mut document = $initial_document;
                let file = File::open($file_name)
                    .expect(concat!("failed to open test file `", $file_name, "`"));
                document
                    .write(
                        file.bytes()
                            .map(|b| b.expect(concat!("could not read file `", $file_name, "`")))
                    )
                    .flush_eof();
                $closure(document);
            }
        };

        (
            $test_name:ident,
            code points: $string:expr,
            $initial_document:expr,
            $expected_document:expr
        ) => {
            #[test]
            fn $test_name() {
                let mut document = $initial_document;
                document.write_chars($string)
                    .flush_chars();
                assert_eq!(document, $expected_document);
            }
        }
    }

    mod charset {
        use super::*;

        file_test! {
            bom_utf8,
            byte stream from "tests/html/docs/charset/bom/utf-8.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::Some(CharEncoding::Utf8));
            }
        }

        file_test! {
            bom_utf16be,
            byte stream from "tests/html/docs/charset/bom/utf-16be.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf16Be));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::Some(CharEncoding::Utf16Be));
            }
        }

        file_test! {
            bom_utf16le,
            byte stream from "tests/html/docs/charset/bom/utf-16le.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf16Le));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::Some(CharEncoding::Utf16Le));
            }
        }

        file_test! {
            meta_charset_utf8,
            byte stream from "tests/html/docs/charset/meta-charset/utf-8.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            meta_charset_utf16be,
            byte stream from "tests/html/docs/charset/meta-charset/utf-16be.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            meta_charset_utf16le,
            byte stream from "tests/html/docs/charset/meta-charset/utf-16le.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        /* TODO
        file_test! {
            meta_charset_windows1252,
            byte stream from "tests/html/docs/charset/meta-charset/windows-1252.html",
            ByteDocument::new(None, None, CharEncoding::Utf8),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Windows1252));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            meta_content_utf8,
            byte stream from "tests/html/docs/charset/meta-content/utf-8.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            meta_content_utf16be,
            byte stream from "tests/html/docs/charset/meta-content/utf-16be.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf16Be));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            meta_content_utf16le,
            byte stream from "tests/html/docs/charset/meta-content/utf-16le.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf16Le));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            meta_content_windows1252,
            byte stream from "tests/html/docs/charset/meta-content/windows-1252.html",
            ByteDocument::new(None, None, CharEncoding::Utf8),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Windows1252));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            xml_encoding_utf8,
            byte stream from "tests/html/docs/charset/xml-encoding/utf-8.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Tentative);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            xml_encoding_utf16be,
            byte stream from "tests/html/docs/charset/xml-encoding/utf-16be.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf16Be));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Tentative);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            xml_encoding_utf16le,
            byte stream from "tests/html/docs/charset/xml-encoding/utf-16le.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf16Le));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Tentative);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            xml_encoding_windows1252,
            byte stream from "tests/html/docs/charset/xml-encoding/windows-1252.html",
            ByteDocument::new(None, None, CharEncoding::Utf8),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Windows1252));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Tentative);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        } */

        // These tests will force the parser to scan for UTF-8, -16BE, and -16LE characters to guess
        // the correct character encoding. This feature isn't supported yet.
        /* TODO
        #[test]
        fn unlabeled_utf8() {
            todo!()
        }

        #[test]
        fn unlabeled_utf16be() {
            todo!()
        }

        #[test]
        fn unlabeled_utf16le() {
            todo!()
        }*/

        /* TODO
        file_test! {
            unrecognized,
            byte stream from "tests/html/docs/charset/infer/unrecognized.html",
            ByteDocument::new(None, None, CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Windows1252));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Tentative);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            override_as_utf8,
            byte stream from "tests/html/docs/empty.html",
            ByteDocument::new(None, Some(CharEncoding::Utf8), CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        }

        file_test! {
            override_as_windows1252,
            byte stream from "tests/html/docs/empty.html",
            ByteDocument::new(None, Some(CharEncoding::Windows1252), CharEncoding::Utf8),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Windows1252));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::None);
            }
        } */

        file_test! {
            bom_trumps_override,
            byte stream from "tests/html/docs/charset/bom/utf-8.html",
            ByteDocument::new(None, Some(CharEncoding::Windows1252), CharEncoding::Windows1252),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Utf8));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::Some(CharEncoding::Utf8));
            }
        }

        /* TODO
        file_test! {
            known_definite_encoding,
            byte stream from "tests/html/docs/charset/bom/utf-8.html",
            ByteDocument::new(Some(CharEncoding::Windows1252), None, CharEncoding::Utf16Be),

            |doc: ByteDocument| {
                assert_eq!(doc.internal.internal.encoding, Some(CharEncoding::Windows1252));
                assert_eq!(doc.internal.internal.enc_confidence, CharEncodingConfidence::Certain);
                assert_eq!(doc.internal.bom_encoding, MaybeCharEncoding::Some(CharEncoding::Utf8));
            }
        }*/
    }

    file_test! {
        text_in_body,
        byte stream from "tests/html/docs/text-in-body.html",
        ByteDocument::new(None, None, CharEncoding::Utf8),

        |doc: ByteDocument| {
            println!("{}", doc.internal.internal.dom);
            assert_eq!(format!("\n{}\n", doc.internal.internal.dom), r#"
[Document]
  [DocumentType name="html" publicId="" systemId=""]
  <[http://www.w3.org/1999/xhtml] html>
    <[http://www.w3.org/1999/xhtml] head>
      "
    "
      <[http://www.w3.org/1999/xhtml] meta charset="utf-8">
      "
  "
    "
  "
    <[http://www.w3.org/1999/xhtml] body>
      "
    Hello, world!
  
"
"#);
        }
    }

    file_test! {
        text_in_inferred_html,
        byte stream from "tests/html/docs/text-in-inferred-html.html",
        ByteDocument::new(None, None, CharEncoding::Utf8),

        |doc: ByteDocument| {
            println!("{}", doc.internal.internal.dom);
            assert_eq!(format!("\n{}\n", doc.internal.internal.dom), r#"
[Document]
  [DocumentType name="html" publicId="" systemId=""]
  <[http://www.w3.org/1999/xhtml] html>
    <[http://www.w3.org/1999/xhtml] head>
      "
  "
      <[http://www.w3.org/1999/xhtml] meta charset="utf-8">
      "
"
    "
"
    <[http://www.w3.org/1999/xhtml] body>
      "
  Hello, world!
"
"#);
        }
    }

    file_test! {
        text_in_inferred_body,
        byte stream from "tests/html/docs/text-in-inferred-body.html",
        ByteDocument::new(None, None, CharEncoding::Utf8),

        |doc: ByteDocument| {
            println!("{}", doc.internal.internal.dom);
            assert_eq!(format!("\n{}\n", doc.internal.internal.dom), r#"
[Document]
  <[http://www.w3.org/1999/xhtml] html>
    <[http://www.w3.org/1999/xhtml] head>
    <[http://www.w3.org/1999/xhtml] body>
      "Hello, world!"
"#);
        }
    }

    file_test! {
        lone_meta,
        byte stream from "tests/html/docs/lone-meta.html",
        ByteDocument::new(None, None, CharEncoding::Utf8),

        |doc: ByteDocument| {
            println!("{}", doc.internal.internal.dom);
            assert_eq!(format!("\n{}\n", doc.internal.internal.dom), r#"
[Document]
  [DocumentType name="html" publicId="" systemId=""]
  <[http://www.w3.org/1999/xhtml] html>
    <[http://www.w3.org/1999/xhtml] head>
      <[http://www.w3.org/1999/xhtml] meta charset="utf-8">
    <[http://www.w3.org/1999/xhtml] body>
"#);
        }
    }

    file_test! {
        lone_meta_end,
        byte stream from "tests/html/docs/lone-meta-end.html",
        ByteDocument::new(None, None, CharEncoding::Utf8),

        |doc: ByteDocument| {
            println!("{}", doc.internal.internal.dom);
            assert_eq!(format!("\n{}\n", doc.internal.internal.dom), r#"
[Document]
  [DocumentType name="html" publicId="" systemId=""]
  <[http://www.w3.org/1999/xhtml] html>
    <[http://www.w3.org/1999/xhtml] head>
    <[http://www.w3.org/1999/xhtml] body>
"#);
        }
    }
}
