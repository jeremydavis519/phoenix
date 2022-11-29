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

//! This module defines the HTML elements.

use alloc::vec::Vec;

use {
    alloc::rc::{Rc, Weak},
    core::{
        cell::RefCell,
        fmt
    },
    crate::{
        DocumentInternal,
        interned_string::InternedString,
        namespace,
        shim::String
    },
    super::node::Node
};

#[derive(Debug)]
pub struct Element<A: alloc::alloc::Allocator+Copy> {
    allocator:                     A,

    pub identifier:                Identifier<A>,
    pub attributes:                Vec<Attribute<A>, A>,
    pub custom_element_state:      CustomElementState,
    pub custom_element_definition: Option<CustomElementDefinition>,
    pub is:                        Option<InternedString<A>>,
    pub children:                  Vec<Rc<RefCell<Node<A>>>, A>,
    pub parent:                    Weak<RefCell<Node<A>>>,
    pub self_closing_acknowledged: bool
}

impl<A: alloc::alloc::Allocator+Copy> Element<A> {
    // https://dom.spec.whatwg.org/#concept-create-element
    pub(super) fn new(
            document:                    &DocumentInternal<A>,
            local_name:                  InternedString<A>,
            namespace:                   &'static str,
            prefix:                      Option<InternedString<A>>,
            is:                          Option<InternedString<A>>,
            synchronous_custom_elements: bool,
            allocator:                   A
    ) -> Rc<RefCell<Node<A>>> {
        let definition = look_up_custom_element_definition(document, namespace, &local_name, &is);
        match definition {
            Some(definition) => todo!(),
            None => {
                // TODO: let interface = element_interface_for(local_name, namespace);
                Rc::new(RefCell::new(Node::Element(Self {
                    allocator,
                    identifier: Identifier {
                        namespace_prefix: prefix,
                        namespace,
                        local_name
                    },
                    attributes: Vec::new_in(allocator),
                    custom_element_state: CustomElementState::Undefined,
                    custom_element_definition: None,
                    is: None,
                    // TODO: interface,
                    // TODO: node_document: Rc::downgrade(document),
                    children: Vec::new_in(allocator),
                    parent:   Weak::new(),
                    self_closing_acknowledged: false
                })))
            }
        }
    }

    pub fn can_insert_child(&self, index: usize) -> bool {
        // FIXME: This shouldn't always be true, but I haven't found where in the spec it's spelled out yet.
        true
    }

    // https://dom.spec.whatwg.org/#concept-element-attributes-append
    pub fn append_attribute(elem: &Rc<RefCell<Node<A>>>, attribute: Attribute<A>) {
        // TODO: "Handle attribute changes for attribute with element, null, and attribute’s value."
        // TODO: attribute.element = Rc::downgrade(elem);
        elem.borrow_mut().as_elem_mut().attributes.push(attribute);
    }

    // https://html.spec.whatwg.org/multipage/forms.html#category-reset
    pub fn is_resettable(&self) -> bool {
        ["input", "output", "select", "textarea"].iter()
                .map(|name| Identifier::new_html(InternedString::from_in(name, self.allocator)))
                .any(|id| id == self.identifier) /* ||
            TODO: "form-associated custom elements" */
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#mathml-text-integration-point
    pub fn is_mathml_text_integration_point(&self) -> bool {
        self.identifier.namespace == namespace::MATHML &&
            ["mi", "mo", "mn", "ms", "mtext"].contains(&self.identifier.local_name.as_str())
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#html-integration-point
    pub fn is_html_integration_point(&self) -> bool {
        (
            self.identifier.namespace == namespace::SVG &&
            ["foreignObject", "desc", "title"].contains(&self.identifier.local_name.as_str())
        ) || (
            self.identifier.namespace == namespace::MATHML &&
            self.identifier.local_name == "annotation-xml" &&
            self.attributes.iter().find(|attr|
                attr.name == "encoding" && (
                    attr.value.eq_ignore_ascii_case("text/html") ||
                    attr.value.eq_ignore_ascii_case("application/xhtml+xml")
                )
            ).is_some()
        )
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#special
    pub fn is_special(&self) -> bool {
        (
            self.identifier.namespace == namespace::HTML &&
            ["address", "applet", "area", "article", "aside", "base", "basefont", "bgsound", "blockquote", "body", "br",
                "button", "caption", "center", "col", "colgroup", "dd", "details", "dir", "div", "dl", "dt", "embed",
                "fieldset", "figcaption", "figure", "footer", "form", "frame", "frameset", "h1", "h2", "h3", "h4", "h5",
                "h6", "head", "header", "hgroup", "hr", "html", "iframe", "img", "input", "keygen", "li", "link",
                "listing", "main", "marquee", "menu", "meta", "nav", "noembed", "noframes", "noscript", "object", "ol",
                "p", "param", "plaintext", "pre", "script", "section", "select", "source", "style", "summary", "table",
                "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "title", "tr", "track", "ul", "wbr",
                "xmp"].contains(&self.identifier.local_name.as_str())
        ) || (
            self.is_mathml_text_integration_point()
        ) || (
            self.identifier.namespace == namespace::SVG &&
            ["foreignObject", "desc", "title"].contains(&self.identifier.local_name.as_str())
        )
    }

    // An implementation of `fmt::Display`, except that it allows an indentation to be specified with
    // the `depth` parameter.
    pub fn display(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        write!(f, "{:indentation$}", "", indentation = depth * 2)?;

        write!(f, "<{}", self.identifier)?;
        if let Some(ref is) = self.is {
            write!(f, " (is {})", is)?;
        }
        for attribute in self.attributes.iter() {
            write!(f, " {}", attribute)?;
        }
        write!(f, ">")?;

        // TODO: Display `self.custom_element_state` and `self.custom_element_definition`?

        for child in self.children.iter() {
            writeln!(f)?;
            child.borrow().display(f, depth + 1)?;
        }

        Ok(())
    }
}

// https://html.spec.whatwg.org/multipage/custom-elements.html#look-up-a-custom-element-definition
pub(super) fn look_up_custom_element_definition<A: alloc::alloc::Allocator+Copy>(
        document:   &DocumentInternal<A>,
        namespace:  &'static str,
        local_name: &InternedString<A>, // FIXME: InternedString should implement Copy.
        is:         &Option<InternedString<A>> // FIXME: InternedString should implement Copy.
) -> Option<CustomElementDefinition> {
    if namespace != namespace::HTML { return None; }
    if document.browsing_context.is_none() { return None; }

    /* TODO
    let registry = &document.relevant_global_object.custom_element_registry;
    if let Some(definition) = registry.get_definition(local_name, local_name) {
        return Some(definition);
    }
    if let Some(definition) = registry.get_definition(is, local_name) {
        return Some(definition);
    } */
    None
}

#[derive(Debug)]
pub struct Identifier<A: alloc::alloc::Allocator> {
    pub namespace_prefix: Option<InternedString<A>>,
    pub namespace: &'static str,
    pub local_name: InternedString<A>
}

impl<A: alloc::alloc::Allocator> PartialEq for Identifier<A> {
    fn eq(&self, other: &Self) -> bool {
        self.local_name == other.local_name &&
            self.namespace == other.namespace &&
            self.namespace_prefix == other.namespace_prefix
    }
}
impl<A: alloc::alloc::Allocator> Eq for Identifier<A> {}

impl<A: alloc::alloc::Allocator> Identifier<A> {
    pub fn new_html(local_name: InternedString<A>) -> Self {
        Self {
            namespace_prefix: None,
            namespace: namespace::HTML,
            local_name
        }
    }

    pub fn new_mathml(local_name: InternedString<A>) -> Self {
        Self {
            namespace_prefix: None,
            namespace: namespace::MATHML,
            local_name
        }
    }

    pub fn new_svg(local_name: InternedString<A>) -> Self {
        Self {
            namespace_prefix: None,
            namespace: namespace::SVG,
            local_name
        }
    }
}

impl<A: alloc::alloc::Allocator> fmt::Display for Identifier<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] ", self.namespace)?;
        if let Some(ref prefix) = self.namespace_prefix {
            write!(f, "{}:", prefix)?;
        }
        write!(f, "{}", self.local_name)
    }
}

#[derive(Debug)]
pub struct Attribute<A: alloc::alloc::Allocator+Copy> {
    // FIXME: pub namespace_prefix: InternedString<A>,
    // FIXME: pub namespace: InternedString<A>,
    pub /* FIXME: local_*/name: InternedString<A>,
    pub value: String<A>,
    // FIXME: pub element: Weak<RefCell<Element>>
}

#[cfg(test)]
impl<A: alloc::alloc::Allocator+Copy> PartialEq for Attribute<A> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

impl<A: alloc::alloc::Allocator+Copy> Attribute<A> {
    pub(super) fn new(allocator: A) -> Self {
        Self {
            name: InternedString::new_in(allocator),
            value: String::new_in(allocator)
        }
    }
}

impl<A: alloc::alloc::Allocator+Copy> fmt::Display for Attribute<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: Display the namespace and namespace prefix as well.
        write!(f, r#"{}="{}""#, self.name, self.value)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CustomElementState {
    Undefined,
    Failed,
    Uncustomized,
    Precustomized,
    Custom
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct CustomElementDefinition;
