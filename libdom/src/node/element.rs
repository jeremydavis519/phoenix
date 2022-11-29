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
        NamedNodeMap,
        descendant_text_content
    },
    crate::{
        idl::{self, DomString, Element as _, NamedNodeMap as _},
        namespace
    }
};

/// Represents a [DOM element](https://dom.spec.whatwg.org/#element).
pub struct Element {
    // https://dom.spec.whatwg.org/#concept-element-namespace
    namespace: Option<Rc<DomString>>,
    // https://dom.spec.whatwg.org/#concept-element-namespace-prefix
    namespace_prefix: Option<Rc<DomString>>,
    // https://dom.spec.whatwg.org/#concept-element-local-name
    local_name: Rc<DomString>,
    // https://dom.spec.whatwg.org/#concept-element-custom-element-state
    custom_element_state: Cell<CustomElementState>,
    // https://dom.spec.whatwg.org/#concept-element-custom-element-definition
    custom_element_definition: (),
    // https://dom.spec.whatwg.org/#concept-element-is-value
    is: Option<()>,
    // https://dom.spec.whatwg.org/#concept-element-attribute
    attribute_list: Rc<NamedNodeMap>,
    // https://dom.spec.whatwg.org/#concept-element-shadow-root
    shadow_root: Option<Rc<dyn idl::ShadowRoot>>,

    // https://dom.spec.whatwg.org/#concept-node-document
    node_document: Weak<dyn idl::Document>,

    // https://dom.spec.whatwg.org/#concept-tree-parent
    parent: Weak<dyn idl::Node>
}

/// Used for defining an element's [custom element state]
/// (https://dom.spec.whatwg.org/#concept-element-custom-element-state).
pub enum CustomElementState {
    Undefined,
    Failed,
    Uncustomized,
    Precustomized,
    Custom
}

impl Element {
    // https://dom.spec.whatwg.org/#concept-element-qualified-name
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

    // https://dom.spec.whatwg.org/#element-html-uppercased-qualified-name
    pub fn html_uppercased_qualified_name(&self) -> Rc<DomString> {
        if self.namespace.as_ref().map(|ns| ns.iter().map(|&c| c).eq(namespace::HTML.encode_utf16())).unwrap_or(false)
                && self.node_document.upgrade().unwrap().is_html() {
            let upper = |domstr: &Rc<DomString>| {
                let mut buf = [0u16; 2];
                let mut new = DomString::with_capacity(domstr.len());
                let old = char::decode_utf16(domstr.iter().map(|&c| c));
                for c in old {
                    match c {
                        Ok(c) => new.extend(c.to_ascii_uppercase().encode_utf16(&mut buf).iter()),
                        Err(e) => new.push(e.unpaired_surrogate())
                    };
                }
                new
            };

            match self.namespace_prefix {
                None => Rc::new(upper(&self.local_name)),
                Some(ref prefix) => {
                    let mut buf = [0u16; 2];
                    let mut name = DomString::with_capacity(prefix.len() + 1 + self.local_name.len());
                    name.extend(upper(prefix));
                    name.extend(':'.encode_utf16(&mut buf).iter());
                    name.extend(upper(&self.local_name));
                    Rc::new(name)
                }
            }
        } else {
            self.qualified_name()
        }
    }
}

impl idl::Element for Element {
    // https://dom.spec.whatwg.org/#dom-element-namespaceuri
    fn namespaceURI(self: Rc<Self>) -> Option<Rc<DomString>> { self.namespace.clone() }
    // https://dom.spec.whatwg.org/#dom-element-prefix
    fn prefix(self: Rc<Self>) -> Option<Rc<DomString>> { self.namespace_prefix.clone() }
    // https://dom.spec.whatwg.org/#dom-element-localname
    fn localName(self: Rc<Self>) -> Rc<DomString> { self.local_name.clone() }
    // https://dom.spec.whatwg.org/#dom-element-tagname
    fn tagName(self: Rc<Self>) -> Rc<DomString> { self.html_uppercased_qualified_name() }

    // https://dom.spec.whatwg.org/#dom-element-id
    // TODO
    fn id(self: Rc<Self>) -> Rc<DomString> { todo!() }
    // TODO
    fn _set_id(self: Rc<Self>, value: Rc<DomString>) { todo!() }
    // https://dom.spec.whatwg.org/#dom-element-classname
    // TODO
    fn className(self: Rc<Self>) -> Rc<DomString> { todo!() }
    // TODO
    fn _set_className(self: Rc<Self>, value: Rc<DomString>) { todo!() }
    // https://dom.spec.whatwg.org/#dom-element-classlist
    // TODO
    fn classList(self: Rc<Self>) -> Rc<dyn idl::DOMTokenList> { todo!() }
    // https://dom.spec.whatwg.org/#dom-element-slot
    // TODO
    fn slot(self: Rc<Self>) -> Rc<DomString> { todo!() }
    // TODO
    fn _set_slot(self: Rc<Self>, value: Rc<DomString>) { todo!() }

    // https://dom.spec.whatwg.org/#dom-element-hasattributes
    fn hasAttributes(self: Rc<Self>) -> bool { self.attribute_list.clone().length() != 0 }
    // https://dom.spec.whatwg.org/#dom-element-attributes
    fn attributes(self: Rc<Self>) -> Rc<dyn idl::NamedNodeMap> { self.attribute_list.clone() }
    // https://dom.spec.whatwg.org/#dom-element-getattributenames
    // TODO
    fn getAttributeNames(self: Rc<Self>) -> Rc<Vec<Rc<DomString>>> { todo!() }

    // https://dom.spec.whatwg.org/#dom-element-getattribute
    // TODO
    fn getAttribute(self: Rc<Self>, qualified_name: Rc<DomString>) -> Option<Rc<DomString>> { todo!() }

    // https://dom.spec.whatwg.org/#dom-element-getattributens
    fn getAttributeNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>)
            -> Option<Rc<DomString>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-setattribute
    fn setAttribute(self: Rc<Self>, qualified_name: Rc<DomString>, value: Rc<DomString>) {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-setattributens
    fn setAttributeNS(
            self: Rc<Self>,
            namespace: Option<Rc<DomString>>,
            local_name: Rc<DomString>,
            value: Rc<DomString>
    ) {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-removeattribute
    fn removeAttribute(self: Rc<Self>, qualified_name: Rc<DomString>) {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-removeattributens
    fn removeAttributeNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>) {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-toggleattribute
    fn toggleAttribute(self: Rc<Self>, qualified_name: Rc<DomString>, force: bool) -> bool {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-hasattribute
    fn hasAttribute(self: Rc<Self>, qualified_name: Rc<DomString>) -> bool {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-hasattributens
    fn hasAttributeNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>) -> bool {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-getattributenode
    fn getAttributeNode(self: Rc<Self>, qualified_name: Rc<DomString>) -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-getattributenodens
    fn getAttributeNodeNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>)
            -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-setattributenode
    fn setAttributeNode(self: Rc<Self>, attr: Rc<dyn idl::Attr>) -> Option<Rc<dyn idl::Attr>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-setattributenodens
    fn setAttributeNodeNS(self: Rc<Self>, attr: Rc<dyn idl::Attr>) -> Option<Rc<dyn idl::Attr>> {
        self.setAttributeNode(attr)
    }

    // https://dom.spec.whatwg.org/#dom-element-removeattributenode
    fn removeAttributeNode(self: Rc<Self>, attr: Rc<dyn idl::Attr>) -> Rc<dyn idl::Attr> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-attachshadow
    fn attachShadow(self: Rc<Self>, init: Rc<idl::ShadowRootInit>) -> Rc<dyn idl::ShadowRoot> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-shadowroot
    fn shadowRoot(self: Rc<Self>) -> Option<Rc<dyn idl::ShadowRoot>> {
        if self.shadow_root.as_ref()?.clone().mode().as_str() == "closed" { return None; }
        self.shadow_root.clone()
    }

    // https://dom.spec.whatwg.org/#dom-element-closest
    fn closest(self: Rc<Self>, selectors: Rc<DomString>) -> Option<Rc<dyn idl::Element>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-matches
    fn matches(self: Rc<Self>, selectors: Rc<DomString>) -> bool {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-webkitmatchesselector
    fn webkitMatchesSelector(self: Rc<Self>, selectors: Rc<DomString>) -> bool {
        self.matches(selectors)
    }

    // https://dom.spec.whatwg.org/#dom-element-getelementsbytagname
    fn getElementsByTagName(self: Rc<Self>, qualified_name: Rc<DomString>) -> Rc<dyn idl::HTMLCollection> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-getelementsbytagnamens
    fn getElementsByTagNameNS(self: Rc<Self>, namespace: Option<Rc<DomString>>, local_name: Rc<DomString>)
            -> Rc<dyn idl::HTMLCollection> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-getelementsbyclassname
    fn getElementsByClassName(self: Rc<Self>, class_names: Rc<DomString>) -> Rc<dyn idl::HTMLCollection> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-insertadjacentelement
    fn insertAdjacentElement(self: Rc<Self>, r#where: Rc<DomString>, element: Rc<dyn idl::Element>)
            -> Option<Rc<dyn idl::Element>> {
        // TODO
        todo!()
    }

    // https://dom.spec.whatwg.org/#dom-element-insertadjacenttext
    fn insertAdjacentText(self: Rc<Self>, r#where: Rc<DomString>, data: Rc<DomString>) {
        // TODO
        todo!()
    }
}

impl idl::Node for Element {
    // https://dom.spec.whatwg.org/#dom-node-nodetype
    fn nodeType(self: Rc<Self>) -> u16 { idl::_Node::ELEMENT_NODE }
    // https://dom.spec.whatwg.org/#dom-node-nodename
    fn nodeName(self: Rc<Self>) -> Rc<DomString> { self.html_uppercased_qualified_name() }

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
    // TODO
    fn hasChildNodes(self: Rc<Self>) -> bool { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-childnodes
    // TODO
    fn childNodes(self: Rc<Self>) -> Rc<dyn idl::NodeList> { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-firstchild
    // TODO
    fn firstChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { todo!() }
    // https://dom.spec.whatwg.org/#dom-node-lastchild
    // TODO
    fn lastChild(self: Rc<Self>) -> Option<Rc<dyn idl::Node>> { todo!() }
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
    fn textContent(self: Rc<Self>) -> Option<Rc<DomString>> { Some(Rc::new(descendant_text_content(&self))) }

    fn _set_textContent(self: Rc<Self>, _value: Option<Rc<DomString>>) {
        // TODO
        todo!()
    }

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
        if other_node.is_none() { return false; }
        let other_node = other_node.unwrap();

        if self.clone().nodeType() != other_node.clone().nodeType() { return false; }
        let other = match other_node.cast::<dyn idl::Element>() {
            Ok(other) => other,
            Err(_) => panic!("implemented interfaces don't match node type")
        };

        if self.clone().namespaceURI() != other.clone().namespaceURI() { return false; }
        if self.clone().prefix() != other.clone().prefix() { return false; }
        if self.clone().localName() != other.clone().localName() { return false; }

        let self_attributes = &self.attribute_list;
        let other_attributes = other.clone().attributes();

        let self_attr_len = self_attributes.clone().length();
        if self_attr_len != other_attributes.clone().length() { return false; }
        for i in 0 .. self_attr_len.try_into().unwrap_or(usize::max_value()) {
            let self_attr = match self_attributes.clone().item(i.try_into().unwrap()) {
                Some(attr) => attr,
                None => unreachable!()
            };
            let self_attr: Rc<dyn idl::Node> = match self_attr.cast() {
                Ok(attr) => attr,
                Err(_) => panic!("non-Node Attr")
            };
            let other_attr = match other_attributes.clone().item(i.try_into().unwrap()) {
                Some(attr) => attr,
                None => unreachable!()
            };
            let other_attr: Rc<dyn idl::Node> = match other_attr.cast() {
                Ok(attr) => attr,
                Err(_) => panic!("non-Node Attr")
            };
            if !self_attr.isEqualNode(Some(other_attr)) { return false; }
        }

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
    fn contains(self: Rc<Self>, other: Option<Rc<dyn idl::Node>>) -> bool {
        match other {
            Some(other) => {
                if ptr::eq(&*self as *const _ as *const u8, &*other as *const _ as *const u8) {
                    true
                } else {
                    for child in self.childNodes()._iter() {
                        if child.clone().contains(Some(other.clone())) {
                            return true;
                        }
                    }
                    false
                }
            },
            None => false
        }
    }

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

impl idl::EventTarget for Element {
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
