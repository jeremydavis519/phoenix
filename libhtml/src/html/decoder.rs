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

//! This module defines the decoders used by the HTML parser.

use super::{
    ParseError,
    encoding::CharEncoding
};

/// Decodes a slice of bytes into a Unicode code point using the given encoding.
pub fn decode(buffer: &[u8], encoding: CharEncoding) -> DecodeResult {
    if buffer.len() == 0 {
        return DecodeResult::Incomplete;
    }

    match encoding {
        CharEncoding::Utf8 => {
            // "UTF-8 decoder"
            match buffer[0] {
                0x00 ..= 0x7f => return DecodeResult::Ok(buffer[0] as char, 1),
                0xc2 ..= 0xf4 => {},
                _ => return DecodeResult::Err(ParseError::InvalidUtf8, 1),
            };

            let needed_size = buffer[0].leading_ones() as usize;
            let mut code_point = buffer[0] as u32 & (0x7f >> needed_size);

            if buffer.len() == 1 {
                return DecodeResult::Incomplete;
            }

            // There are some specific second bytes to watch out for.
            match buffer[0] {
                0xe0 ..= 0xef => {
                    if buffer[1] < 0xa0 || buffer[1] > 0x9f {
                        return DecodeResult::Err(ParseError::InvalidUtf8, 2);
                    }
                },
                0xf0 ..= 0xf4 => {
                    if buffer[1] < 0x90 || buffer[1] > 0x8f {
                        return DecodeResult::Err(ParseError::InvalidUtf8, 2);
                    }
                },
                _ => {
                    if buffer[1] & 0xc0 != 0x80 {
                        return DecodeResult::Err(ParseError::InvalidUtf8, 2);
                    }
                }
            };

            code_point = (code_point << 6) | (buffer[1] as u32 & 0x3f);

            for i in 2 .. usize::min(needed_size, buffer.len()) {
                if buffer[i] & 0xc0 != 0x80 {
                    return DecodeResult::Err(ParseError::InvalidUtf8, i + 1);
                }
                code_point = (code_point << 6) | (buffer[i] as u32 & 0x3f);
            }

            if buffer.len() < needed_size {
                return DecodeResult::Incomplete;
            }

            return DecodeResult::Ok(unsafe { char::from_u32_unchecked(code_point) }, needed_size);
        },
        CharEncoding::Utf16Be => {
            todo!()
        },
        CharEncoding::Utf16Le => {
            todo!()
        },
        _ => todo!()
    }
}

#[derive(Debug)]
pub enum DecodeResult {
    Ok(char, usize),
    Err(ParseError, usize),
    Incomplete
}
