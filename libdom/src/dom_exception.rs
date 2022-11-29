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

use {
    alloc::rc::Rc,
    crate::{
        idl::{self, DOMString},
        Record
    }
};

pub struct DomException {
    name: Rc<DOMString>,
    message: Rc<DOMString>,
    code: u16
}

impl idl::DOMException for DomException {
    // https://webidl.spec.whatwg.org/#dom-domexception-domexception
    fn constructor(&mut self, message: Rc<DOMString>, name: Rc<DOMString>) {
        self.name = name;
        self.message = message;
    }

    // https://webidl.spec.whatwg.org/#dom-domexception-name
    fn name(self: Rc<Self>) -> Rc<DOMString> { self.name.clone() }
    // https://webidl.spec.whatwg.org/#dom-domexception-message
    fn message(self: Rc<Self>) -> Rc<DOMString> { self.message.clone() }

    // https://webidl.spec.whatwg.org/#dom-domexception-code
    fn code(self: Rc<Self>) -> u16 {
        [
            ("IndexSizeError".encode_utf16().collect::<DOMString>(), idl::_DOMException::INDEX_SIZE_ERR),
            ("HierarchyRequestError".encode_utf16().collect::<DOMString>(), idl::_DOMException::HIERARCHY_REQUEST_ERR),
            ("WrongDocumentError".encode_utf16().collect::<DOMString>(), idl::_DOMException::WRONG_DOCUMENT_ERR),
            ("InvalidCharacterError".encode_utf16().collect::<DOMString>(), idl::_DOMException::INVALID_CHARACTER_ERR),
            ("NoModificationAllowedError".encode_utf16().collect::<DOMString>(),
                idl::_DOMException::NO_MODIFICATION_ALLOWED_ERR),
            ("NotFoundError".encode_utf16().collect::<DOMString>(), idl::_DOMException::NOT_FOUND_ERR),
            ("NotSupportedError".encode_utf16().collect::<DOMString>(), idl::_DOMException::NOT_SUPPORTED_ERR),
            ("InUseAttributeError".encode_utf16().collect::<DOMString>(), idl::_DOMException::INUSE_ATTRIBUTE_ERR),
            ("InvalidStateError".encode_utf16().collect::<DOMString>(), idl::_DOMException::INVALID_STATE_ERR),
            ("SyntaxError".encode_utf16().collect::<DOMString>(), idl::_DOMException::SYNTAX_ERR),
            ("InvalidModificationError".encode_utf16().collect::<DOMString>(),
                idl::_DOMException::INVALID_MODIFICATION_ERR),
            ("NamespaceError".encode_utf16().collect::<DOMString>(), idl::_DOMException::NAMESPACE_ERR),
            ("InvalidAccessError".encode_utf16().collect::<DOMString>(), idl::_DOMException::INVALID_ACCESS_ERR),
            ("TypeMismatchError".encode_utf16().collect::<DOMString>(), idl::_DOMException::TYPE_MISMATCH_ERR),
            ("SecurityError".encode_utf16().collect::<DOMString>(), idl::_DOMException::SECURITY_ERR),
            ("NetworkError".encode_utf16().collect::<DOMString>(), idl::_DOMException::NETWORK_ERR),
            ("AbortError".encode_utf16().collect::<DOMString>(), idl::_DOMException::ABORT_ERR),
            ("URLMismatchError".encode_utf16().collect::<DOMString>(), idl::_DOMException::URL_MISMATCH_ERR),
            ("QuotaExceededError".encode_utf16().collect::<DOMString>(), idl::_DOMException::QUOTA_EXCEEDED_ERR),
            ("TimeoutError".encode_utf16().collect::<DOMString>(), idl::_DOMException::TIMEOUT_ERR),
            ("InvalidNodeTypeError".encode_utf16().collect::<DOMString>(), idl::_DOMException::INVALID_NODE_TYPE_ERR),
            ("DataCloneError".encode_utf16().collect::<DOMString>(), idl::_DOMException::DATA_CLONE_ERR)
        ]
            .iter()
            .find(|&(s, _)| *s == *self.name)
            .map(|&(_, code)| code)
            .unwrap_or(0)
    }

    fn _serialize(&self, serialized: Record, _for_storage: bool) -> Result<(), Rc<dyn idl::DOMException>> {
        // TODO
        todo!()
    }

    fn _deserialize(&mut self, serialized: Record) -> Result<(), Rc<dyn idl::DOMException>> {
        // TODO
        todo!()
    }
}
