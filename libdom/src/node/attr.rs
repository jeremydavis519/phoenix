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
        string::String,
        vec::Vec
    },
    core::{
        cell::RefCell,
        ptr
    },
    intertrait::cast::CastRc,
    super::idl::{self, DomString}
};

/// Represents a DOM [attribute](https://dom.spec.whatwg.org/#interface-attr).
pub struct Attr {
    // https://dom.spec.whatwg.org/#concept-attribute-namespace
    namespace: Option<Rc<DomString>>,
    // https://dom.spec.whatwg.org/#concept-attribute-namespace-prefix
    namespace_prefix: Option<Rc<DomString>>,
    // https://dom.spec.whatwg.org/#concept-attribute-local-name
    local_name: Rc<DomString>,
    // https://dom.spec.whatwg.org/#concept-attribute-value
    value: RefCell<Rc<DomString>>,
    // https://dom.spec.whatwg.org/#concept-attribute-element
    element: Weak<dyn idl::Element>,

    // https://dom.spec.whatwg.org/#concept-node-document
    node_document: Weak<dyn idl::Document>
}

impl Attr {
    pub fn qualified_name(&self) -> Rc<DomString> {
        match self.namespace_prefix {
            None => self.local_name.clone(),
            Some(ref prefix) => {
                let mut buf = [0u16; 2];
                let mut name = DomString::with_capacity(prefix.len() + 1 + self.local_name.len());
                name.extend(prefix.iter());
                name.extend(':'.encode_utf16(&mut buf).iter());
                name.extend(self.local_name.iter());
                Rc::new(name)
            }
        }
    }
}

impl idl::Attr for Attr {
    // https://dom.spec.whatwg.org/#dom-attr-namespaceuri
    fn namespaceURI(self: Rc<Self>) -> Option<Rc<DomString>> { self.namespace.clone() }
    // https://dom.spec.whatwg.org/#dom-attr-prefix
    fn prefix(self: Rc<Self>) -> Option<Rc<DomString>> { self.namespace_prefix.clone() }
    // https://dom.spec.whatwg.org/#dom-attr-localname
    fn localName(self: Rc<Self>) -> Rc<DomString> { self.local_name.clone() }
    // https://dom.spec.whatwg.org/#dom-attr-name
    fn name(self: Rc<Self>) -> Rc<DomString> { self.qualified_name() }
    // https://dom.spec.whatwg.org/#dom-attr-value
    fn value(self: Rc<Self>) -> Rc<DomString> { self.value.borrow().clone() }

    fn _set_value(self: Rc<Self>, value: Rc<DomString>) {
        match self.element.upgrade() {
            None => *self.value.borrow_mut() = value,
            Some(element) => {
                // TODO
                todo!()
            }
        }
    }

    // https://dom.spec.whatwg.org/#dom-attr-ownerelement
    fn ownerElement(self: Rc<Self>) -> Option<Rc<dyn idl::Element>> { self.element.upgrade() }

    // https://dom.spec.whatwg.org/#dom-attr-specified
    fn specified(self: Rc<Self>) -> bool { true }
}

impl idl::Node for Attr {
    // https://dom.spec.whatwg.org/#dom-node-nodetype
    fn nodeType(self: Rc<Self>) -> u16 { idl::_Node::ATTRIBUTE_NODE }
    // https://dom.spec.whatwg.org/#dom-node-nodename
    fn nodeName(self: Rc<Self>) -> Rc<DomString> { self.qualified_name() }

    // https://dom.spec.whatwg.org/#dom-node-baseuri
    // TODO
    fn baseURI(self: Rc<Self>) -> Rc<String> { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-isconnected
    // TODO
    fn isConnected(self: Rc<Self>) -> bool { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-ownerdocument
    fn ownerDocument(self: Rc<Self>) -> Option<Rc<dyn idl::Document>> { self.node_document.upgrade() }
    // https://dom.spec.whatwg.org/#dom-node-getrootnode
    fn getRootNode(self: Rc<Self>, options: Rc<idl::GetRootNodeOptions>) -> Rc<dyn idl::Node> { self.clone() }
    // https://dom.spec.whatwg.org/#dom-node-parentnode
    fn parentNode(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }
    // https://dom.spec.whatwg.org/#dom-node-parentelement
    fn parentElement(self: Rc<Self>) -> Option<Rc<dyn idl::Element>> { None }
    // https://dom.spec.whatwg.org/#dom-node-haschildnodes
    fn hasChildNodes(self: Rc<Self>) -> bool { false }
    // https://dom.spec.whatwg.org/#dom-node-childnodes
    fn childNodes(self: Rc<Self>) -> Rc<dyn idl::NodeList> { Rc::new(Vec::<Rc<dyn idl::Node>>::new()) }
    // https://dom.spec.whatwg.org/#dom-node-firstchild
    fn firstChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }
    // https://dom.spec.whatwg.org/#dom-node-lastchild
    fn lastChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }
    // https://dom.spec.whatwg.org/#dom-node-previoussibling
    fn previousSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }
    // https://dom.spec.whatwg.org/#dom-node-nextsibling
    fn nextSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }

    // https://dom.spec.whatwg.org/#dom-node-nodevalue
    fn nodeValue(self: Rc<Self>) -> Option<Rc<DomString>> { Some(self.value.borrow().clone()) }
    fn _set_nodeValue(self: Rc<Self>, value: Option<Rc<DomString>>) {
        use idl::Attr;
        self._set_value(value.unwrap_or(Rc::new(DomString::new())))
    }
    // https://dom.spec.whatwg.org/#dom-node-textcontent
    fn textContent(self: Rc<Self>) -> Option<Rc<DomString>> { Some(self.value.borrow().clone()) }
    fn _set_textContent(self: Rc<Self>, value: Option<Rc<DomString>>) {
        use idl::Attr;
        self._set_value(value.unwrap_or(Rc::new(DomString::new())))
    }
    // https://dom.spec.whatwg.org/#dom-node-normalize
    fn normalize(self: Rc<Self>) {}

    // https://dom.spec.whatwg.org/#dom-node-clonenode
    fn cloneNode(self: Rc<Self>, _deep: bool) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-isequalnode
    fn isEqualNode(self: Rc<Self>, other_node: Option<Rc<dyn idl::Node>>) -> bool {
        use idl::Attr;

        if other_node.is_none() { return false; }
        let other_node = other_node.unwrap();

        if self.clone().nodeType() != other_node.clone().nodeType() { return false; }
        let other = match other_node.cast::<dyn idl::Attr>() {
            Ok(other) => other,
            Err(_) => panic!("implemented interfaces don't match node type")
        };

        if self.clone().namespaceURI() != other.clone().namespaceURI() { return false; }
        if self.clone().prefix() != other.clone().prefix() { return false; }
        if self.localName() != other.clone().localName() { return false; }

        !other.hasChildNodes()
    }

    // https://dom.spec.whatwg.org/#dom-node-issamenode
    fn isSameNode(self: Rc<Self>, other_node: Option<Rc<dyn idl::Node>>) -> bool {
        match other_node {
            Some(other) => ptr::eq(&*self as *const _ as *const u8, &*other as *const _ as *const u8),
            None => false
        }
    }

    // https://dom.spec.whatwg.org/#dom-node-comparedocumentposition
    fn compareDocumentPosition(self: Rc<Self>, _other: Rc<dyn idl::Node>) -> u16 {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-contains
    fn contains(self: Rc<Self>, other: Option<Rc<dyn idl::Node>>) -> bool { false }

    // https://dom.spec.whatwg.org/#dom-node-lookupprefix
    fn lookupPrefix(self: Rc<Self>, namespace: Option<Rc<DomString>>) -> Option<Rc<DomString>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-lookupnamespaceuri
    fn lookupNamespaceURI(self: Rc<Self>, prefix: Option<Rc<DomString>>) -> Option<Rc<DomString>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-isdefaultnamespace
    fn isDefaultNamespace(self: Rc<Self>, namespace: Option<Rc<DomString>>) -> bool {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-insertbefore
    fn insertBefore(self: Rc<Self>, node: Rc<dyn idl::Node>, child: Option<Rc<dyn idl::Node>>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-appendchild
    fn appendChild(self: Rc<Self>, node: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        self.insertBefore(node, None)
    }

    // https://dom.spec.whatwg.org/#dom-node-replacechild
    fn replaceChild(self: Rc<Self>, node: Rc<dyn idl::Node>, child: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-removechild
    fn removeChild(self: Rc<Self>, node: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }
}

impl idl::EventTarget for Attr {
    // https://dom.spec.whatwg.org/#dom-eventtarget-eventtarget
    fn constructor(&mut self) {}

    // https://dom.spec.whatwg.org/#dom-eventtarget-addeventlistener
    fn addEventListener(
        self: Rc<Self>,
        _type:   Rc<DomString>,
        _callback: Option<Rc<dyn idl::EventListener>>,
        _options:  idl::_EventTarget::_Union_AddEventListenerOptions_or_boolean
    ) {
        // TODO
        todo!();
    }

    // https://dom.spec.whatwg.org/#dom-eventtarget-removeeventlistener
    fn removeEventListener(
        self: Rc<Self>,
        _type:   Rc<DomString>,
        _callback: Option<Rc<dyn idl::EventListener>>,
        _options:  idl::_EventTarget::_Union_EventListenerOptions_or_boolean
    ) {
        // TODO
        todo!();
    }

    // https://dom.spec.whatwg.org/#dom-eventtarget-dispatchevent
    fn dispatchEvent(self: Rc<Self>, _event: Rc<dyn idl::Event>) -> bool {
        // TODO
        todo!();
    }
}
