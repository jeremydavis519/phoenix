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
    alloc::{
        rc::{Rc, Weak},
        vec::Vec
    },
    super::idl::{self, DomString}
};

/// Represents a [`NamedNodeMap`](https://dom.spec.whatwg.org/#namednodemap)
pub struct NamedNodeMap {
    element: Weak<dyn idl::Element>,
    attribute_list: Rc<Vec<Rc<dyn idl::Attr>>>
}

impl idl::NamedNodeMap for NamedNodeMap {
    // https://dom.spec.whatwg.org/#dom-namednodemap-length
    fn length(self: Rc<Self>) -> u32 {
        u32::try_from(self.attribute_list.len()).expect("NamedNodeMap length overflow")
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-item
    fn item(self: Rc<Self>, index: u32) -> Option<Rc<dyn idl::Attr>> {
        self.attribute_list.get(usize::try_from(index).expect("NamedNodeMap index overflow"))
            .map(|rc| rc.clone())
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-getnameditem
    fn getNamedItem(self: Rc<Self>, qualified_name: Rc<DomString>) -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-getnameditemns
    fn getNamedItemNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>)
            -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-setnameditem
    fn setNamedItem(self: Rc<Self>, attr: Rc<dyn idl::Attr>) -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-setnameditemns
    fn setNamedItemNS(self: Rc<Self>, attr: Rc<dyn idl::Attr>) -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-removenameditem
    fn removeNamedItem(self: Rc<Self>, qualified_name: Rc<DomString>) -> Rc<dyn idl::Attr> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-namednodemap-removenameditemns
    fn removeNamedItemNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>)
            -> Rc<dyn idl::Attr> {
        // TODO
        todo!()
    }
}
