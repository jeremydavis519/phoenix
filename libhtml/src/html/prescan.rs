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

//! Here the parser can prescan a byte stream to determine its most likely encoding.

use {
    crate::ByteDocumentInternal,
    super::{
        ParseResult,
        encoding::{
            CharEncoding,
            CharEncodingConfidence,
            MaybeCharEncoding
        }
    }
};

// The "encoding sniffing algorithm"
pub(super) fn sniff_char_encoding<A: alloc::alloc::Allocator+Copy>(
    document: &mut ByteDocumentInternal<A>
) -> ParseResult<(CharEncoding, CharEncodingConfidence), !> {
    match parse_bom(document) {
        ParseResult::Ok(encoding) => {
            // We found a valid byte order mark, and we now know the proper encoding.
            return ParseResult::Ok((encoding, CharEncodingConfidence::Certain));
        },
        ParseResult::Later => {
            // We still need another byte or two to finish the BOM.
            return ParseResult::Later;
        },
        ParseResult::Err(()) => {}
    };

    if let Some(encoding) = document.user_encoding {
        return ParseResult::Ok((encoding, CharEncodingConfidence::Certain));
    }

    // TODO: "If the transport layer specifies a character encoding, and it is supported,
    // return that encoding with the confidence certain." (This is different than the
    // _irrelevant_ case, as that bypasses even the BOM check.)

    match prescan_for_encoding(document) {
        ParseResult::Ok(encoding) => {
            return ParseResult::Ok((encoding, CharEncodingConfidence::Tentative));
        },
        // FIXME: Are there any byte streams that lead to an infinite ParseResult::Later loop?
        ParseResult::Later => return ParseResult::Later,
        ParseResult::Err(()) => {}
    };

    // TODO: "If the HTML parser for which this algorithm is being run is associated with a
    // Document d whose browsing context is non-null and a child browsing context, then:
    //  1. Let parentDocument be d's browsing context's container document.
    //  2. If parentDocument's origin is same origin with d's origin and parentDocument's
    //     character encoding is not UTF-16BE/LE, then return parentDocument's character
    //     encoding, with the confidence tentative."

    // TODO: "Otherwise, if the user agent has information on the likely encoding for this
    // page, e.g. based on the encoding of the page when it was last visited, then return
    // that encoding, with the confidence tentative."

    // TODO: "The user agent may attempt to autodetect the character encoding from applying
    // frequency analysis or other algorithms to the data stream. Such algorithms may use
    // information about the resource other than the resource's contents, including the
    // address of the resource. If autodetection succeeds in determining a character
    // encoding, and that encoding is a supported encoding, then return that encoding, with
    // the confidence tentative." (This is most likely to work well when the whole file is
    // available, especially if we're checking for UTF-8.)

    ParseResult::Ok((document.default_encoding, CharEncodingConfidence::Tentative))
}

pub(super) fn parse_bom<A: alloc::alloc::Allocator+Copy>(
        document: &mut ByteDocumentInternal<A>
) -> ParseResult<CharEncoding, ()> {
    match document.bom_encoding {
        MaybeCharEncoding::Some(encoding) => {
            // We've already sniffed and consumed the BOM.
            ParseResult::Ok(encoding)
        },
        MaybeCharEncoding::None => {
            // We've already checked and found no BOM.
            ParseResult::Err(())
        },
        MaybeCharEncoding::Tbd => {
            // We haven't checked yet.
            let bytes = document.byte_queue.make_contiguous();

            static UTF8_BOM: [u8; 3] = [0xef, 0xbb, 0xbf];
            static UTF16BE_BOM: [u8; 2] = [0xfe, 0xff];
            static UTF16LE_BOM: [u8; 2] = [0xff, 0xfe];

            if bytes.starts_with(&UTF8_BOM) {
                for _ in 0 .. 3 { document.byte_queue.pop_front(); }
                document.bom_encoding = MaybeCharEncoding::Some(CharEncoding::Utf8);
                return ParseResult::Ok(CharEncoding::Utf8);
            }
            if bytes.starts_with(&UTF16BE_BOM) {
                for _ in 0 .. 2 { document.byte_queue.pop_front(); }
                document.bom_encoding = MaybeCharEncoding::Some(CharEncoding::Utf16Be);
                return ParseResult::Ok(CharEncoding::Utf16Be);
            }
            if bytes.starts_with(&UTF16LE_BOM) {
                for _ in 0 .. 2 { document.byte_queue.pop_front(); }
                document.bom_encoding = MaybeCharEncoding::Some(CharEncoding::Utf16Le);
                return ParseResult::Ok(CharEncoding::Utf16Le);
            }

            if UTF8_BOM.starts_with(bytes) ||
                    UTF16BE_BOM.starts_with(bytes) ||
                    UTF16LE_BOM.starts_with(bytes) {
                // We only have the prefix of a valid BOM.
                return ParseResult::Later;
            }

            document.bom_encoding = MaybeCharEncoding::None;
            ParseResult::Err(())
        }
    }
}

// "Prescan a byte stream to determine its encoding"
// Attempts to determine the byte stream's encoding from whatever bytes we already have. Not
// guaranteed to be correct.
fn prescan_for_encoding<A: alloc::alloc::Allocator+Copy>(
        document: &mut ByteDocumentInternal<A>
) -> ParseResult<CharEncoding, ()> {
    const MAX_PRESCAN_LENGTH: usize = 1024;
    let bytes = clamped_slice(document.byte_queue.make_contiguous(), 0, MAX_PRESCAN_LENGTH);
    let mut position = 0;

    // Prescan for UTF-16 XML declarations.
    match clamped_slice(bytes, position, position + 6) {
        b"<\0?\0x\0" => return ParseResult::Ok(CharEncoding::Utf16Le),
        b"\0<\0?\0x" => return ParseResult::Ok(CharEncoding::Utf16Be),
        _ => {}
    };

    while position < bytes.len() {
        if clamped_slice(bytes, position, position + 4) == b"<!--" {
            // <!-- ... -->
            match bytes.windows(3).skip(position).position(|w| w == b"-->") {
                Some(p) => {
                    position += p + 3;
                    continue;
                },
                None => break
            };
        }

        match clamped_slice(bytes, position, position + 6) {
            &[b'<', m, e, t, a, x]
                    if m.to_ascii_lowercase() == b'm' &&
                       e.to_ascii_lowercase() == b'e' &&
                       t.to_ascii_lowercase() == b't' &&
                       a.to_ascii_lowercase() == b'a' &&
                       x.is_ascii_whitespace() || x == b'/' => {
                // <meta>
                position += 5;
                let mut found_http_equiv = false;
                let mut found_content = false;
                let mut found_charset = false;
                let mut got_pragma = false;
                let mut need_pragma = None;
                let mut charset = None;

                // Parse the relevant attributes.
                loop {
                    match get_attribute(&mut position, bytes) {
                        Ok((attr_name, attr_value)) => {
                            if attr_name.eq_ignore_ascii_case(b"http-equiv") {
                                if !found_http_equiv {
                                    found_http_equiv = true;
                                    if attr_value.eq_ignore_ascii_case(b"content-type") {
                                        got_pragma = true;
                                    }
                                }
                            } else if attr_name.eq_ignore_ascii_case(b"content") {
                                if !found_content {
                                    found_content = true;
                                    if charset.is_none() {
                                        match CharEncoding::from_meta(attr_value) {
                                            Ok(encoding) => {
                                                charset = Some(encoding);
                                                need_pragma = Some(true);
                                            },
                                            Err(()) => {}
                                        };
                                    }
                                }
                            } else if attr_name.eq_ignore_ascii_case(b"charset") {
                                if !found_charset {
                                    found_charset = true;
                                    charset = CharEncoding::from_label(attr_value).ok();
                                    need_pragma = Some(false);
                                }
                            }
                        },
                        Err(()) => break
                    };
                }

                // Final processing
                match (need_pragma, got_pragma) {
                    (Some(false), _) | (Some(true), true) => {
                        match charset {
                            Some(CharEncoding::Utf16Be) | Some(CharEncoding::Utf16Le) => {
                                return ParseResult::Ok(CharEncoding::Utf8);
                            },
                            Some(CharEncoding::XUserDefined) => {
                                return ParseResult::Ok(CharEncoding::Windows1252)
                            },
                            Some(encoding) => {
                                return ParseResult::Ok(encoding);
                            },
                            None => {
                                // This tag didn't specify a character set.
                                position += 1;
                                continue;
                            }
                        };
                    },
                    _ => {
                        // Either we needed the pragma and didn't find it or this tag didn't
                        // specify a character set.
                        position += 1;
                        continue;
                    }
                }
            },
            _ => {}
        };

        match clamped_slice(bytes, position, position + 3) {
            &[b'<', a] |
            &[b'<', a, _] |
            &[b'<', b'/', a] if a.is_ascii_alphabetic() => {
                // A regular opening or closing tag
                match bytes.iter().skip(position).position(|&b| b.is_ascii_whitespace() || b == b'>') {
                    Some(p) => {
                        position += p;
                    },
                    None => break
                };
                // We don't care about the attributes, so just parse and discard them.
                loop {
                    match get_attribute(&mut position, bytes) {
                        Ok((_, _)) => {},
                        Err(()) => break
                    };
                }
                position += 1;
                continue;
            },
            _ => {}
        };

        match clamped_slice(bytes, position, position + 2) {
            b"<!" | b"</" | b"<?" => {
                // <!...>, </...>, or <?...>
                match bytes.iter().skip(position).position(|&b| b == b'>') {
                    Some(p) => {
                        position += p + 1;
                        continue;
                    },
                    None => break
                }
            },
            _ => {}
        };

        // Not a tag
        position += 1;
    }

    get_xml_encoding(document)
}

// "Get an XML encoding"
fn get_xml_encoding<A: alloc::alloc::Allocator+Copy>(
        document: &mut ByteDocumentInternal<A>
) -> ParseResult<CharEncoding, ()> {
    const MAX_PRESCAN_LENGTH: usize = 1024;
    let bytes = clamped_slice(document.byte_queue.make_contiguous(), 0, MAX_PRESCAN_LENGTH);

    let mut position = 0;

    if clamped_slice(bytes, position, position + 5) != b"<?xml" {
        return ParseResult::Err(());
    }

    let xml_declaration_end = match bytes.iter().skip(position).position(|&b| b == b'>') {
        Some(p) => position + p,
        None => return ParseResult::Err(())
    };
    let bytes = &bytes[0 .. xml_declaration_end];

    match bytes.windows(8).skip(position).position(|w| w == b"encoding") {
        Some(p) => position += p + 8,
        None => return ParseResult::Err(())
    };

    match bytes.iter().skip(position).position(|&b| b > 0x20) {
        Some(p) => position += p,
        None => return ParseResult::Err(())
    };

    if bytes[position] != b'=' {
        return ParseResult::Err(());
    }
    position += 1;

    match bytes.iter().skip(position).position(|&b| b > 0x20) {
        Some(p) => position += p,
        None => return ParseResult::Err(())
    };

    let quote_mark = bytes[position];
    if quote_mark != b'"' && quote_mark != b'\'' {
        return ParseResult::Err(());
    }

    position += 1;

    let end_position = match bytes.iter().skip(position).position(|&b| b == quote_mark) {
        Some(p) => position + p,
        None => return ParseResult::Err(())
    };

    let potential_encoding = &bytes[position .. end_position];
    if potential_encoding.iter().any(|&b| b <= 0x20) {
        return ParseResult::Err(());
    }

    ParseResult::from(CharEncoding::from_label(potential_encoding))
}

// "Get an attribute"
// Returns the name and value of an attribute, as subslices of the `bytes` slice.
// NOTE: The specification describes converting the attribute's name and value to lowercase as
//       part of this algorithm, but we can't do that in-place without modifying the byte
//       stream. Instead, we have to make sure that every use of the returned name and value is
//       case-insensitive.
fn get_attribute<'a>(
    position: &mut usize,
    bytes: &'a [u8]
) -> Result<(&'a [u8], &'a [u8]), ()> {
    match bytes.iter().skip(*position).position(|&b| !b.is_ascii_whitespace() && b != b'/') {
        Some(p) => *position += p,
        None => {
            *position = bytes.len();
            return Err(());
        }
    };

    if bytes[*position] == b'>' {
        return Err(());
    }

    // Get the attribute's name.
    let attr_name_start = *position;
    while *position < bytes.len() {
        match bytes[*position] {
            b'=' if *position > attr_name_start => break,
            x if x.is_ascii_whitespace() => break,
            b'/' | b'>' => {
                // There's no equals sign, so the attribute's value is the empty string.
                let attr_name = &bytes[attr_name_start .. *position];
                let attr_value = &bytes[0 .. 0];
                return Ok((attr_name, attr_value));
            },
            _ => {
                // Add the character to the attribute name.
                *position += 1;
            }
        };
    }
    let attr_name = &bytes[attr_name_start .. *position];

    // Whitespace and the equals sign
    match bytes.iter().skip(*position).position(|&b| !b.is_ascii_whitespace()) {
        Some(p) => *position += p,
        None => {
            // There's no equals sign, so the attribute's value is the empty string.
            let attr_value = &bytes[0 .. 0];
            return Ok((attr_name, attr_value));
        }
    };

    if bytes[*position] != b'=' {
        // There's no equals sign, so the attribute's value is the empty string.
        let attr_value = &bytes[0 .. 0];
        return Ok((attr_name, attr_value));
    }
    *position += 1;

    match bytes.iter().skip(*position).position(|&b| !b.is_ascii_whitespace()) {
        Some(p) => *position += p,
        None => {
            // There's nothing after the equals sign, so the attribute's value is the empty string.
            let attr_value = &bytes[0 .. 0];
            return Ok((attr_name, attr_value));
        }
    };

    // Get the attribute's value.
    let attr_value_start;
    match bytes[*position] {
        quote @ b'"' | quote @ b'\'' => {
            attr_value_start = *position + 1;
            loop {
                *position += 1;
                if *position >= bytes.len() {
                    // We've run out of bytes before the end of the value. The only safe thing to
                    // do is to throw it out.
                    return Err(());
                }
                if bytes[*position] == quote {
                    // We've found the end of the value.
                    let attr_value = &bytes[attr_value_start .. *position];
                    return Ok((attr_name, attr_value));
                }
            }
        },
        b'>' => {
            // There's nothing after the equals sign, so the attribute's value is the empty string.
            let attr_value = &bytes[0 .. 0];
            return Ok((attr_name, attr_value));
        },
        _ => {
            // This is the start of an unquoted value.
            attr_value_start = *position;
            *position += 1;
        }
    };

    match bytes.iter().skip(*position).position(|&b| b.is_ascii_whitespace() || b == b'>') {
        Some(p) => {
            // We've found the end of the value.
            *position += p;
            let attr_value = &bytes[attr_value_start .. *position];
            return Ok((attr_name, attr_value));
        },
        None => {
            // We've run out of bytes before confirming the end of the value. The only safe thing to
            // do is to throw it out.
            return Err(());
        }
    };
}

// Slices the source slice after forcing the start and end indices to be in-bounds.
fn clamped_slice<T>(src: &[T], start: usize, end: usize) -> &[T] {
    let len = src.len();
    &src[usize::min(start, len) .. usize::min(end, len)]
}
