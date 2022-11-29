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
        iter,
        ptr
    },
    intertrait::cast::CastRc,
    super::{
        Comment,
        DocumentFragment,
        NamedNodeMap,
        Text,
        descendant_text_content,
        replace_data,
        substring_data,
        previous_sibling,
        next_sibling
    },
    crate::{
        idl::{self, DomString, Document as _},
        namespace
    }
};

struct Encoding; // FIXME: Properly define the encodings. https://encoding.spec.whatwg.org/#encoding
struct Url; // FIXME: Properly define a URL. https://url.spec.whatwg.org/#concept-url
struct Origin; // FIXME: Properly define an origin. https://html.spec.whatwg.org/multipage/origin.html#concept-origin

// https://dom.spec.whatwg.org/#concept-document-type
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DocumentType {
    Xml,
    Html
}

// https://dom.spec.whatwg.org/#concept-document-mode
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DocumentMode {
    NoQuirks,
    Quirks,
    LimitedQuirks
}

/// Represents a [`Document` node](https://dom.spec.whatwg.org/#interface-document).
pub struct Document {
    // https://dom.spec.whatwg.org/#concept-document-encoding
    encoding: Encoding,

    // https://dom.spec.whatwg.org/#concept-document-content-type
    content_type: Rc<DomString>,

    // https://dom.spec.whatwg.org/#concept-document-url
    url: Url,

    // https://dom.spec.whatwg.org/#concept-document-origin
    origin: Origin,

    // https://dom.spec.whatwg.org/#concept-document-type
    ty: DocumentType,

    // https://dom.spec.whatwg.org/#concept-document-mode
    mode: DocumentMode,

    // https://dom.spec.whatwg.org/#concept-tree-parent
    parent: Weak<dyn idl::Node>,
    
    children: Rc<Vec<Rc<dyn idl::Node>>>
}

impl Document {
    fn new(
            encoding: Encoding,
            content_type: DomString,
            url: Url,
            ty: DocumentType,
            mode: DocumentMode,
            parent: Weak<dyn idl::Node>
    ) -> Self {
        let mut doc = Document {
            encoding,
            content_type: Rc::new(content_type),
            url,
            origin: Origin,
            ty,
            mode,
            parent,
            children: Rc::new(Vec::new())
        };
        doc.constructor();
        doc
    }
}

impl idl::Document for Document {
    // https://dom.spec.whatwg.org/#dom-document-document
    fn constructor(&mut self) {
        // TODO
        self.origin = todo!();
    }

    // https://dom.spec.whatwg.org/#dom-document-implementation
    // TODO
    fn implementation(self: Rc<Self>) -> Rc<dyn idl::DOMImplementation> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-url
    // TODO
    fn URL(self: Rc<Self>) -> Rc<String> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-documenturi
    fn documentURI(self: Rc<Self>) -> Rc<String> { self.URL() }

    // https://dom.spec.whatwg.org/#dom-document-compatmode
    fn compatMode(self: Rc<Self>) -> Rc<DomString> {
        match self.mode {
            DocumentMode::Quirks => Rc::new("BackCompat".encode_utf16().collect()),
            _                    => Rc::new("CSS1Compat".encode_utf16().collect())
        }
    }

    // https://dom.spec.whatwg.org/#dom-document-characterset
    // TODO
    fn characterSet(self: Rc<Self>) -> Rc<DomString> { todo!() }
    // https://dom.spec.whatwg.org/#dom-document-charset
    fn charset(self: Rc<Self>) -> Rc<DomString> { self.characterSet() }
    // https://dom.spec.whatwg.org/#dom-document-inputencoding
    fn inputEncoding(self: Rc<Self>) -> Rc<DomString> { self.characterSet() }

    // https://dom.spec.whatwg.org/#dom-document-contenttype
    fn contentType(self: Rc<Self>) -> Rc<DomString> { self.content_type.clone() }

    // https://dom.spec.whatwg.org/#dom-document-doctype
    // TODO
    fn doctype(self: Rc<Self>) -> Option<Rc<dyn idl::DocumentType>> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-documentelement
    fn documentElement(self: Rc<Self>) -> Option<Rc<dyn idl::Element>> {
        for child in self.children.iter() {
            if let Ok(elem) = child.clone().cast::<dyn idl::Element>() {
                return Some(elem);
            }
        }
        None
    }

    // https://dom.spec.whatwg.org/#dom-document-getelementsbytagname
    // TODO
    fn getElementsByTagName(self: Rc<Self>, _qualified_name: Rc<DomString>) -> Rc<dyn idl::HTMLCollection> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-getelementsbytagnamens
    // TODO
    fn getElementsByTagNameNS(self: Rc<Self>, _namespace: Option<Rc<DomString>>, _local_name: Rc<DomString>)
            -> Rc<dyn idl::HTMLCollection> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-getelementsbyclassname
    // TODO
    fn getElementsByClassName(self: Rc<Self>, _class_names: Rc<DomString>) -> Rc<dyn idl::HTMLCollection> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-createelement
    // TODO
    fn createElement(
            self: Rc<Self>,
            _local_name: Rc<DomString>,
            _options: idl::_Document::_Union_DOMString_or_ElementCreationOptions
    ) -> Rc<dyn idl::Element> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-createelementns
    fn createElementNS(
            self: Rc<Self>,
            _namespace: Option<Rc<DomString>>,
            _qualified_name: Rc<DomString>,
            _options: idl::_Document::_Union_DOMString_or_ElementCreationOptions
    ) -> Rc<dyn idl::Element> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-createdocumentfragment
    fn createDocumentFragment(self: Rc<Self>) -> Rc<dyn idl::DocumentFragment> {
        Rc::new(DocumentFragment::new(None, Rc::downgrade(&self).clone()))
    }

    // https://dom.spec.whatwg.org/#dom-document-createtextnode
    fn createTextNode(self: Rc<Self>, data: Rc<DomString>) -> Rc<dyn idl::Text> {
        let mut text = Text::new(data);
        text.node_document = Rc::downgrade(&self).clone();
        Rc::new(text)
    }

    // https://dom.spec.whatwg.org/#dom-document-createcdatasection
    // TODO
    fn createCDATASection(self: Rc<Self>, _data: Rc<DomString>) -> Rc<dyn idl::CDATASection> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-createcomment
    fn createComment(self: Rc<Self>, data: Rc<DomString>) -> Rc<dyn idl::Comment> {
        let mut comment = Comment::new(data);
        comment.node_document = Rc::downgrade(&self).clone();
        Rc::new(comment)
    }

    // https://dom.spec.whatwg.org/#dom-document-createprocessinginstruction
    // TODO
    fn createProcessingInstruction(self: Rc<Self>, _target: Rc<DomString>, _data: Rc<DomString>)
            -> Rc<dyn idl::ProcessingInstruction> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-importnode
    // TODO
    fn importNode(self: Rc<Self>, _node: Rc<dyn idl::Node>, _deep: bool) -> Rc<dyn idl::Node> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-adoptnode
    // TODO
    fn adoptNode(self: Rc<Self>, _node: Rc<dyn idl::Node>) -> Rc<dyn idl::Node> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-createattribute
    // TODO
    fn createAttribute(self: Rc<Self>, _local_name: Rc<DomString>) -> Rc<dyn idl::Attr> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-createattributens
    // TODO
    fn createAttributeNS(self: Rc<Self>, _namespace: Option<Rc<DomString>>, _qualified_name: Rc<DomString>)
            -> Rc<dyn idl::Attr> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-createevent
    // TODO
    fn createEvent(self: Rc<Self>, _interface: Rc<DomString>) -> Rc<dyn idl::Event> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-createrange
    // TODO
    fn createRange(self: Rc<Self>) -> Rc<dyn idl::Range> { todo!() }

    // https://dom.spec.whatwg.org/#dom-document-createnodeiterator
    // TODO
    fn createNodeIterator(
            self: Rc<Self>,
            _root: Rc<dyn idl::Node>,
            _what_to_show: u32,
            _filter: Option<Rc<dyn idl::NodeFilter>>
    ) -> Rc<dyn idl::NodeIterator> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-document-createtreewalker
    // TODO
    fn createTreeWalker(
            self: Rc<Self>,
            _root: Rc<dyn idl::Node>,
            _what_to_show: u32,
            _filter: Option<Rc<dyn idl::NodeFilter>>
    ) -> Rc<dyn idl::TreeWalker> {
        todo!()
    }

    // https://dom.spec.whatwg.org/#html-document
    fn is_html(self: Rc<Self>) -> bool { self.ty == DocumentType::Html }
    // https://dom.spec.whatwg.org/#xml-document
    fn is_xml(self: Rc<Self>) -> bool { self.ty == DocumentType::Xml }
    
}

impl idl::Node for Document {
    // https://dom.spec.whatwg.org/#dom-node-nodetype
    fn nodeType(self: Rc<Self>) -> u16 { idl::_Node::DOCUMENT_NODE }
    // https://dom.spec.whatwg.org/#dom-node-nodename
    fn nodeName(self: Rc<Self>) -> Rc<DomString> { Rc::new("#document".encode_utf16().collect()) }

    // https://dom.spec.whatwg.org/#dom-node-baseuri
    // TODO
    fn baseURI(self: Rc<Self>) -> Rc<String> { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-isconnected
    fn isConnected(self: Rc<Self>) -> bool {
        self.getRootNode(Rc::new(idl::GetRootNodeOptions { composed: false })).cast::<dyn idl::Document>().is_ok()
    }
    // https://dom.spec.whatwg.org/#dom-node-ownerdocument
    fn ownerDocument(self: Rc<Self>) -> Option<Rc<dyn idl::Document>> { None }

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
    fn hasChildNodes(self: Rc<Self>) -> bool { !self.children.is_empty() }
    // https://dom.spec.whatwg.org/#dom-node-childnodes
    fn childNodes(self: Rc<Self>) -> Rc<dyn idl::NodeList> { self.children.clone() }
    // https://dom.spec.whatwg.org/#dom-node-firstchild
    fn firstChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { self.children.first().map(|rc| rc.clone()) }
    // https://dom.spec.whatwg.org/#dom-node-lastchild
    fn lastChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { self.children.last().map(|rc| rc.clone()) }
    // https://dom.spec.whatwg.org/#dom-node-previoussibling
    fn previousSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { previous_sibling(&self) }
    // https://dom.spec.whatwg.org/#dom-node-nextsibling
    fn nextSibling(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { next_sibling(&self) }

    // https://dom.spec.whatwg.org/#dom-node-nodevalue
    fn nodeValue(self: Rc<Self>) -> Option<Rc<DomString>> { None }
    fn _set_nodeValue(self: Rc<Self>, _value: Option<Rc<DomString>>) {}
    // https://dom.spec.whatwg.org/#dom-node-textcontent
    fn textContent(self: Rc<Self>) -> Option<Rc<DomString>> { None }
    fn _set_textContent(self: Rc<Self>, _value: Option<Rc<DomString>>) {}

    // https://dom.spec.whatwg.org/#dom-node-normalize
    // TODO
    fn normalize(self: Rc<Self>) { todo!() }

    // https://dom.spec.whatwg.org/#dom-node-clonenode
    fn cloneNode(self: Rc<Self>, _deep: bool) -> Rc<dyn idl::Node> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-node-isequalnode
    fn isEqualNode(self: Rc<Self>, other_node: Option<Rc<dyn idl::Node>>) -> bool {
        if other_node.is_none() { return false; }
        let other_node = other_node.unwrap();

        if self.clone().nodeType() != other_node.clone().nodeType() { return false; }
        let other = match other_node.cast::<dyn idl::Document>() {
            Ok(other) => other,
            Err(_) => panic!("implemented interfaces don't match node type")
        };

        let self_children = self.childNodes();
        let other_children = other.childNodes();
        if self_children.clone().length() != other_children.clone().length() { return false; }
        for i in 0 .. self_children.clone().length() {
            let self_child = match self_children.clone().item(i) {
                Some(child) => child,
                None => unreachable!()
            };
            let other_child = match other_children.clone().item(i) {
                Some(child) => child,
                None => unreachable!()
            };
            if !self_child.isEqualNode(Some(other_child)) { return false; }
        }

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
    // TODO
    fn contains(self: Rc<Self>, other: Option<Rc<dyn idl::Node>>) -> bool { todo!() }

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

impl idl::EventTarget for Document {
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
