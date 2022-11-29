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

//! This library defines an HTML parser and DOM hierarchy that are compliant with the [HTML Living
//! Standard](http://html.spec.whatwg.org).

#![cfg_attr(not(test), no_std)]

#![deny(missing_docs)]

#![feature(allocator_api)]
#![feature(never_type)]

extern crate alloc;

use {
    alloc::{
        collections::vec_deque::VecDeque,
        rc::Rc
    },
    core::cell::RefCell,
    html::{
        encoding::{
            CharEncodingConfidence,
            MaybeCharEncoding
        },
        node::Node
    }
};

mod html;
pub mod interned_string;
mod namespace;
mod shim;

/// A document, as defined by the specification. This object contains the entire DOM and allows new
/// text to be appended at the end, which is then immediately parsed and added to the DOM.
///
/// This type expects text to be delivered in a byte stream. [`CharDocument`] should be preferred if
/// you have a stream of Unicode code points instead (e.g. a `&str` in Rust).
#[derive(Debug)]
pub struct ByteDocument<A: alloc::alloc::Allocator+Copy = alloc::alloc::Global> {
    parser: html::Parser<A>,
    tokenizer: html::tokenizer::Tokenizer<A>,
    internal: ByteDocumentInternal<A>
}

#[derive(Debug)]
struct ByteDocumentInternal<A: alloc::alloc::Allocator+Copy> {
    byte_queue: VecDeque<u8, A>,

    // The character encoding according to a BOM found at the beginning of the file (takes precedence
    // over everything else for the purpose of decoding the byte stream).
    bom_encoding: MaybeCharEncoding,
    // The character encoding the user has requested (trumps everything except a "known definite
    // encoding" or a BOM).
    user_encoding: Option<CharEncoding>,
    // The character encoding to use if all attempts at detection fail.
    default_encoding: CharEncoding,

    internal: DocumentInternal<A>
}

/// A document, as defined by the specification. This object contains the entire DOM and allows new
/// text to be appended at the end, which is then immediately parsed and added to the DOM.
///
/// This type expects text to be delivered in a stream of Unicode code points (e.g. a `&str` in
/// Rust). If you have a byte stream instead, use [`ByteDocument`].
#[derive(Debug)]
pub struct CharDocument<A: alloc::alloc::Allocator+Copy = alloc::alloc::Global> {
    parser: html::Parser<A>,
    tokenizer: html::tokenizer::Tokenizer<A>,
    internal: CharDocumentInternal<A>
}

#[derive(Debug)]
struct CharDocumentInternal<A: alloc::alloc::Allocator+Copy> {
    char_queue: VecDeque<char, A>,

    internal: DocumentInternal<A>
}

#[derive(Debug)]
struct DocumentInternal<A: alloc::alloc::Allocator+Copy> {
    // The DOM tree
    dom: html::dom::Dom<A>,

    // The DOCTYPE defined at the top of the file, if any
    document_type: Option<Rc<RefCell<Node<A>>>>,

    // The level of "quirks mode" that the document needs
    quirks_mode: QuirksMode,

    // True if this document is specified in the `srcdoc` attribute of an iframe.
    is_iframe_srcdoc: bool,

    // The browsing context whose session history includes the document, if any.
    browsing_context: Option<()>,

    // The character encoding, once we've determined it. If one is not provided by the client
    // program, this is `None`, and the first parsing step is to determine a likely encoding.
    encoding: Option<CharEncoding>,
    enc_confidence: CharEncodingConfidence,

    // https://html.spec.whatwg.org/multipage/parsing.html#head-element-pointer
    head_element: Option<Rc<RefCell<Node<A>>>>
}

pub use html::encoding::CharEncoding;


impl ByteDocument {
    /// Constructs a new document with the global allocator.
    pub fn new(
            certain_encoding: Option<CharEncoding>,
            user_encoding:    Option<CharEncoding>,
            default_encoding: CharEncoding
    ) -> Self {
        Self::new_with_allocator(certain_encoding, user_encoding, default_encoding, alloc::alloc::Global)
    }
}

impl<A: alloc::alloc::Allocator+Copy> ByteDocument<A> {
    /// Constructs a new document with the given allocator.
    pub fn new_with_allocator(
            certain_encoding: Option<CharEncoding>,
            user_encoding:    Option<CharEncoding>,
            default_encoding: CharEncoding,
            allocator:        A
    ) -> Self {
        let encoding;
        let enc_confidence;
        if certain_encoding.is_some() {
            encoding = certain_encoding;
            enc_confidence = CharEncodingConfidence::Certain;
        } else {
            encoding = None;
            enc_confidence = CharEncodingConfidence::Tentative;
        }

        Self {
            parser: html::Parser::new(false, allocator),
            tokenizer: html::tokenizer::Tokenizer::new(allocator),
            internal: ByteDocumentInternal {
                byte_queue: VecDeque::new_in(allocator),
                bom_encoding: MaybeCharEncoding::Tbd,
                user_encoding,
                default_encoding,
                internal: DocumentInternal::new(encoding, enc_confidence, allocator)
            }
        }
    }

    /// Writes the given bytes to the document. Note that they are not parsed until [`flush`] is
    /// called.
    pub fn write<I>(&mut self, bytes: I) -> &mut Self
            where I: IntoIterator<Item = u8> {
        self.internal.byte_queue.extend(bytes);
        self
    }

    /// Flushes to the parser any bytes that have been written so far.
    pub fn flush(&mut self) -> &mut Self {
        self.parser.flush_byte_stream(&mut self.tokenizer, &mut self.internal, false);
        self
    }

    /// Flushes to the parser any bytes that have been written so far, followed by an EOF.
    pub fn flush_eof(&mut self) -> &mut Self {
        self.parser.flush_byte_stream(&mut self.tokenizer, &mut self.internal, true);
        self
    }
}

impl CharDocument {
    /// Constructs a new document with the global allocator.
    pub fn new() -> Self {
        Self::new_with_allocator(alloc::alloc::Global)
    }
}

impl<A: alloc::alloc::Allocator+Copy> CharDocument<A> {
    /// Constructs a new document with the given allocator.
    pub fn new_with_allocator(allocator: A) -> Self {
        Self {
            parser: html::Parser::new(false, allocator),
            tokenizer: html::tokenizer::Tokenizer::new(allocator),
            internal: CharDocumentInternal {
                char_queue: VecDeque::new_in(allocator),
                internal: DocumentInternal::new(None, CharEncodingConfidence::Irrelevant, allocator)
            }
        }
    }

    /// Writes the given characters to the document. Note that they are not parsed until [`flush`]
    /// is called.
    pub fn write<I>(&mut self, chars: I) -> &mut Self
            where I: IntoIterator<Item = char> {
        self.internal.char_queue.extend(chars);
        self
    }

    /// Flushes to the parser any characters that have been written so far.
    pub fn flush(&mut self) -> &mut Self {
        self.parser.flush_char_stream(&mut self.tokenizer, &mut self.internal, false);
        self
    }

    /// Flushes to the parser any characters that have been written so far, followed by an EOF.
    pub fn flush_eof(&mut self) -> &mut Self {
        self.parser.flush_char_stream(&mut self.tokenizer, &mut self.internal, true);
        self
    }
}

impl<A: alloc::alloc::Allocator+Copy> DocumentInternal<A> {
    fn new(encoding: Option<CharEncoding>, enc_confidence: CharEncodingConfidence, allocator: A) -> Self {
        Self {
            dom: html::dom::Dom::new(allocator),
            document_type: None,
            quirks_mode: QuirksMode::NoQuirks,
            is_iframe_srcdoc: false,
            browsing_context: None,
            encoding,
            enc_confidence,
            head_element: None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum QuirksMode {
    NoQuirks,
    LimitedQuirks,
    Quirks
}
