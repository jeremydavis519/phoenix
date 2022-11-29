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
    core::ptr,
    intertrait::cast::CastRc,
    super::idl::{self, DomString}
};

/// Represents a [`DocumentType`](https://dom.spec.whatwg.org/#documenttype) node.
pub struct DocumentType {
    // https://dom.spec.whatwg.org/#concept-doctype-name
    name:      Rc<DomString>,
    // https://dom.spec.whatwg.org/#concept-doctype-publicid
    public_id: Rc<DomString>,
    // https://dom.spec.whatwg.org/#concept-doctype-systemid
    system_id: Rc<DomString>,

    // https://dom.spec.whatwg.org/#concept-node-document
    node_document: Weak<dyn idl::Document>,

    // https://dom.spec.whatwg.org/#concept-tree-parent
    parent: Weak<dyn idl::Node>
}

impl DocumentType {
    pub fn new(
            name: DomString,
            public_id: DomString,
            system_id: DomString,
            node_document: Weak<dyn idl::Document>,
            parent: Weak<dyn idl::Node>
    ) -> Self {
        let mut doctype = Self {
            name: Rc::new(name),
            public_id: Rc::new(public_id),
            system_id: Rc::new(system_id),
            node_document,
            parent
        };
        <dyn idl::Node>::constructor(&mut doctype);
        doctype
    }
}

impl idl::DocumentType for DocumentType {
    // https://dom.spec.whatwg.org/#dom-documenttype-name
    fn name(self: Rc<Self>) -> Rc<DomString> { self.name.clone() }
    // https://dom.spec.whatwg.org/#dom-documenttype-publicid
    fn publicId(self: Rc<Self>) -> Rc<DomString> { self.public_id.clone() }
    // https://dom.spec.whatwg.org/#dom-documenttype-systemid
    fn systemId(self: Rc<Self>) -> Rc<DomString> { self.system_id.clone() }
}

impl idl::Node for DocumentType {
    // https://dom.spec.whatwg.org/#dom-node-nodetype
    fn nodeType(self: Rc<Self>) -> u16 { idl::_Node::DOCUMENT_TYPE_NODE }
    // https://dom.spec.whatwg.org/#dom-node-nodename
    fn nodeName(self: Rc<Self>) -> Rc<DomString> { self.name.clone() }

    // https://dom.spec.whatwg.org/#dom-node-baseuri
    // TODO
    fn baseURI(self: Rc<Self>) -> Rc<String> { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-isconnected
    // TODO
    fn isConnected(self: Rc<Self>) -> bool { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-ownerdocument
    fn ownerDocument(self: Rc<Self>) -> Option<Rc<dyn idl::Document>> { self.node_document.upgrade() }

    // https://dom.spec.whatwg.org/#dom-node-getrootnode
    fn getRootNode(self: Rc<Self>, _options: Rc<idl::GetRootNodeOptions>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-parentnode
    fn parentNode(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { self.parent.upgrade() }

    // https://dom.spec.whatwg.org/#dom-node-parentelement
    fn parentElement(self: Rc<Self>) -> Option<Rc<dyn idl::Element>> {
        self.parentNode().map(|node| node.cast().ok()).flatten()
    }

    // https://dom.spec.whatwg.org/#dom-node-haschildnodes
    fn hasChildNodes(self: Rc<Self>) -> bool { false }
    // https://dom.spec.whatwg.org/#dom-node-childnodes
    fn childNodes(self: Rc<Self>) -> Rc<dyn idl::NodeList> { Rc::new(Vec::<Rc<dyn idl::Node>>::new()) }
    // https://dom.spec.whatwg.org/#dom-node-firstchild
    fn firstChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }
    // https://dom.spec.whatwg.org/#dom-node-lastchild
    fn lastChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { None }
    // https://dom.spec.whatwg.org/#dom-node-previoussibling
    // TODO
    fn previousSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-nextsibling
    // TODO
    fn nextSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-nodevalue
    fn nodeValue(self: Rc<Self>) -> Option<Rc<DomString>> { None }
    fn _set_nodeValue(self: Rc<Self>, _value: Option<Rc<DomString>>) {}
    // https://dom.spec.whatwg.org/#dom-node-textcontent
    fn textContent(self: Rc<Self>) -> Option<Rc<DomString>> { None }
    fn _set_textContent(self: Rc<Self>, _value: Option<Rc<DomString>>) {}

    // https://dom.spec.whatwg.org/#dom-node-normalize
    fn normalize(self: Rc<Self>) {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-clonenode
    fn cloneNode(self: Rc<Self>, _deep: bool) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-isequalnode
    fn isEqualNode(self: Rc<Self>, other_node: Option<Rc<dyn idl::Node>>) -> bool {
        use idl::DocumentType;

        if other_node.is_none() { return false; }
        let other_node = other_node.unwrap();

        if self.clone().nodeType() != other_node.clone().nodeType() { return false; }
        let other = match other_node.cast::<dyn idl::DocumentType>() {
            Ok(other) => other,
            Err(_) => panic!("implemented interfaces don't match node type")
        };

        if self.clone().name() != other.clone().name() { return false; }
        if self.clone().publicId() != other.clone().publicId() { return false; }
        if self.systemId() != other.clone().systemId() { return false; }

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
    fn contains(self: Rc<Self>, other: Option<Rc<dyn idl::Node>>) -> bool {
        match other {
            // No recursion is necessary because a doctype can't have children.
            Some(other) => ptr::eq(&*self as *const _ as *const u8, &*other as *const _ as *const u8),
            None => false
        }
    }

    // https://dom.spec.whatwg.org/#dom-node-lookupprefix
    fn lookupPrefix(self: Rc<Self>, _namespace: Option<Rc<DomString>>) -> Option<Rc<DomString>> { None }
    // https://dom.spec.whatwg.org/#dom-node-lookupnamespaceuri
    fn lookupNamespaceURI(self: Rc<Self>, _prefix: Option<Rc<DomString>>) -> Option<Rc<DomString>> { None }

    // https://dom.spec.whatwg.org/#dom-node-isdefaultnamespace
    fn isDefaultNamespace(self: Rc<Self>, namespace: Option<Rc<DomString>>) -> bool {
        namespace.is_none() || namespace.unwrap().is_empty()
    }

    // https://dom.spec.whatwg.org/#dom-node-insertbefore
    fn insertBefore(self: Rc<Self>, _node: Rc<dyn idl::Node>, _child: Option<Rc<dyn idl::Node>>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-appendchild
    fn appendChild(self: Rc<Self>, node: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        self.insertBefore(node, None)
    }

    // https://dom.spec.whatwg.org/#dom-node-replacechild
    fn replaceChild(self: Rc<Self>, _node: Rc<dyn idl::Node>, _child: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-removechild
    fn removeChild(self: Rc<Self>, _child: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }
}

impl idl::EventTarget for DocumentType {
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
