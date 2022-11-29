/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines the Document Object Model, the standard representation of Web pages in a browser. It
//! conforms to the specification at [https://dom.spec.whatwg.org/].

#![no_std]

extern crate alloc;

use alloc::{
    string::String,
    vec::Vec
};

// TODO: Maybe use a hash map for this instead.
// A record is a set of named values. The values have their types erased.
type Record = Vec<(String, Vec<u8>)>;

pub mod idl {
    // TODO: Extend the `include_idl` macro to handle every IDL file in this directory at once (`include_idl_dir`?).

    use idl2rust::{
        define_idl_types,
        include_idl,
    };

    define_idl_types!();

    // https://html.spec.whatwg.org/multipage/structured-data.html#serializable
    macro_rules! xattr_Serializable {
        ([Serializable] pub trait $trait_name:ident : $(:: $supertype1:ident)+ $(+ $(:: $supertypes:ident)+)* {
            $($items:item)*
        }) => {
            pub trait $trait_name : $(:: $supertype1)+ $(+ $(:: $supertypes)+)* {
                $($items)*

                fn _serialize(
                    self: Rc<Self>,
                    serialized: <$crate>::Record,
                    forStorage: bool
                ) where Self: Sized
                    -> Result<(), ::alloc::rc::Rc<dyn $crate::idl::DOMException>>;

                fn _deserialize(
                    self: Rc<Self>,
                    serialized: <$crate>::Record
                ) where Self: Sized
                    -> Result<::alloc::rc::Rc<Self>, ::alloc::rc::Rc<dyn $crate::idl::DOMException>>;
            }
        }
    }

    macro_rules! xattr_CEReactions {
        ([CEReactions] $($tts:tt)*) => { $($tts)* };
    }

    macro_rules! xattr_PhoenixUA {
        ([PhoenixUA] $($tts:tt)*) => { $($tts)* };
    }

    include_idl!("AbortSignal.idl");
    include_idl!("AbstractRange.idl");
    include_idl!("Attr.idl");
    include_idl!("CDATASection.idl");
    include_idl!("CharacterData.idl");
    include_idl!("Comment.idl");
    include_idl!("Document.idl");
    include_idl!("DocumentFragment.idl");
    include_idl!("DocumentType.idl");
    include_idl!("DOMException.idl");
    include_idl!("DOMHighResTimeStamp.idl");
    include_idl!("DOMImplementation.idl");
    include_idl!("DOMTokenList.idl");
    include_idl!("Element.idl");
    include_idl!("Event.idl");
    include_idl!("EventHandler.idl");
    include_idl!("EventTarget.idl");
    include_idl!("HTMLCollection.idl");
    include_idl!("NamedNodeMap.idl");
    include_idl!("Node.idl");
    include_idl!("NodeFilter.idl");
    include_idl!("NodeIterator.idl");
    include_idl!("NodeList.idl");
    include_idl!("ProcessingInstruction.idl");
    include_idl!("Range.idl");
    include_idl!("ShadowRoot.idl");
    include_idl!("Text.idl");
    include_idl!("TreeWalker.idl");
}

pub mod dom_exception;
pub mod namespace;
pub mod node;
