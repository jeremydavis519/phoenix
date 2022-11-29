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
        cell::Cell,
        ptr
    },
    intertrait::cast::CastRc,
    super::{
        Document,
        NamedNodeMap,
        descendant_text_content,
        do_cloning_steps,
        replace_data,
        substring_data,
        previous_sibling,
        next_sibling
    },
    crate::{
        idl::{self, DomString, CharacterData as _, Comment as _},
        namespace
    }
};

/// Represents a [`Comment` node](https://dom.spec.whatwg.org/#interface-comment).
pub struct Comment {
    // https://dom.spec.whatwg.org/#concept-cd-data
    data: Rc<DomString>,

    // https://dom.spec.whatwg.org/#concept-node-document
    pub(super) node_document: Weak<dyn idl::Document>,

    // https://dom.spec.whatwg.org/#concept-tree-parent
    parent: Weak<dyn idl::Node>
}

impl Comment {
    pub(super) fn new(data: Rc<DomString>) -> Self {
        let mut comment = Comment {
            data: data.clone(),
            node_document: Weak::<Document>::new(),
            parent: Weak::<Self>::new()
        };
        comment.constructor(data);
        comment
    }
}

impl idl::Comment for Comment {
    // https://dom.spec.whatwg.org/#dom-comment-comment
    fn constructor(&mut self, data: Rc<DomString>) {
        self.data = data;
        // TODO
        self.node_document = todo!();
    }
}

impl idl::CharacterData for Comment {
    // https://dom.spec.whatwg.org/#dom-characterdata-data
    fn data(self: Rc<Self>) -> Rc<DomString> { self.data.clone() }
    fn _set_data(self: Rc<Self>, value: Rc<DomString>) {
        replace_data(self.clone(), 0, self.length(), &value)
    }
    // https://dom.spec.whatwg.org/#dom-characterdata-length
    fn length(self: Rc<Self>) -> u32 { self.data.len().try_into().expect("Comment length overflow") }
    // https://dom.spec.whatwg.org/#dom-characterdata-substringdata
    fn substringData(self: Rc<Self>, offset: u32, count: u32) -> Rc<DomString> {
        substring_data(self, offset, count)
    }
    // https://dom.spec.whatwg.org/#dom-characterdata-appenddata
    fn appendData(self: Rc<Self>, data: Rc<DomString>) {
        replace_data(self.clone(), self.length(), 0, &data)
    }
    // https://dom.spec.whatwg.org/#dom-characterdata-insertdata
    fn insertData(self: Rc<Self>, offset: u32, data: Rc<DomString>) {
        replace_data(self, offset, 0, &data)
    }
    // https://dom.spec.whatwg.org/#dom-characterdata-deletedata
    fn deleteData(self: Rc<Self>, offset: u32, count: u32) {
        replace_data(self, offset, count, &Rc::new(DomString::new()))
    }
    // https://dom.spec.whatwg.org/#dom-characterdata-replacedata
    fn replaceData(self: Rc<Self>, offset: u32, count: u32, data: Rc<DomString>) {
        replace_data(self, offset, count, &data)
    }
}

impl idl::Node for Comment {
    // https://dom.spec.whatwg.org/#dom-node-nodetype
    fn nodeType(self: Rc<Self>) -> u16 { idl::_Node::TEXT_NODE }
    // https://dom.spec.whatwg.org/#dom-node-nodename
    fn nodeName(self: Rc<Self>) -> Rc<DomString> { Rc::new("#text".encode_utf16().collect()) }

    // https://dom.spec.whatwg.org/#dom-node-baseuri
    // TODO
    fn baseURI(self: Rc<Self>) -> Rc<String> { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-isconnected
    fn isConnected(self: Rc<Self>) -> bool {
        self.getRootNode(Rc::new(idl::GetRootNodeOptions { composed: false })).cast::<dyn idl::Document>().is_ok()
    }
    // https://dom.spec.whatwg.org/#dom-node-ownerdocument
    fn ownerDocument(self: Rc<Self>) -> Option<Rc<dyn idl::Document>> { self.node_document.upgrade() }

    // https://dom.spec.whatwg.org/#dom-node-getrootnode
    fn getRootNode(self: Rc<Self>, options: Rc<idl::GetRootNodeOptions>) -> Rc<dyn idl::Node> {
        match self.clone().parentNode() {
            Some(parent) => parent.getRootNode(options),
            None => self
        }
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
    fn previousSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { previous_sibling(&self) }
    // https://dom.spec.whatwg.org/#dom-node-nextsibling
    fn nextSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { next_sibling(&self) }

    // https://dom.spec.whatwg.org/#dom-node-nodevalue
    fn nodeValue(self: Rc<Self>) -> Option<Rc<DomString>> { Some(self.data.clone()) }
    fn _set_nodeValue(self: Rc<Self>, value: Option<Rc<DomString>>) {
        self._set_textContent(value)
    }
    // https://dom.spec.whatwg.org/#dom-node-textcontent
    fn textContent(self: Rc<Self>) -> Option<Rc<DomString>> { Some(self.data.clone()) }
    fn _set_textContent(self: Rc<Self>, value: Option<Rc<DomString>>) {
        replace_data(self.clone(), 0, self.length(), &value.unwrap_or(Rc::new(DomString::new())))
    }

    // https://dom.spec.whatwg.org/#dom-node-normalize
    fn normalize(self: Rc<Self>) {}

    // https://dom.spec.whatwg.org/#dom-node-clonenode
    fn cloneNode(self: Rc<Self>, deep: bool) -> Rc<dyn idl::Node> {
        // (Steps 1 and 2 only apply to documents and elements.)

        // 3. Let copy be a node that implements the same interfaces as node [...].
        // Set copy’s data to that of node.
        let mut copy = Comment::new(self.data.clone());

        // 4. Set copy’s node document to document.
        copy.node_document = self.node_document.clone();

        // 5. Run any cloning steps defined for node in other applicable specifications and pass copy, node, document
        // and the clone children flag if set, as parameters.
        let copy = Rc::new(copy);
        unsafe {
            do_cloning_steps(
                copy.clone(),
                self.clone(),
                self.node_document.upgrade().expect("missing node document"),
                deep
            );
        }

        // (Step 6 only applies to nodes with children.)

        // 7. Return copy.
        copy
    }

    // https://dom.spec.whatwg.org/#dom-node-isequalnode
    fn isEqualNode(self: Rc<Self>, other_node: Option<Rc<dyn idl::Node>>) -> bool {
        if other_node.is_none() { return false; }
        let other_node = other_node.unwrap();

        if self.clone().nodeType() != other_node.clone().nodeType() { return false; }
        let other = match other_node.cast::<dyn idl::Comment>() {
            Ok(other) => other,
            Err(_) => panic!("implemented interfaces don't match node type")
        };

        if self.data() != other.data() { return false; }

        true
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
    fn contains(self: Rc<Self>, other: Option<Rc<dyn idl::Node>>) -> bool { self.isSameNode(other) }

    // https://dom.spec.whatwg.org/#dom-node-lookupprefix
    // TODO
    fn lookupPrefix(self: Rc<Self>, _namespace: Option<Rc<DomString>>) -> Option<Rc<DomString>> { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-lookupnamespaceuri
    // TODO
    fn lookupNamespaceURI(self: Rc<Self>, _prefix: Option<Rc<DomString>>) -> Option<Rc<DomString>> { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-isdefaultnamespace
    fn isDefaultNamespace(self: Rc<Self>, namespace: Option<Rc<DomString>>) -> bool {
        // TODO
        todo!()
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

impl idl::EventTarget for Comment {
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
