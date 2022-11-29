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

//! This module defines the type that represents an HTML node.

use {
    alloc::{
        rc::{Rc, Weak},
        vec::Vec
    },
    core::{
        cell::RefCell,
        fmt
    },
    crate::shim::String,
    super::{
        dom::Dom,
        element::Element,
        tokenizer
    }
};

// TODO: Write a WebIDL parser to generate these from the appropriate interface definitions.

#[derive(Debug)]
pub enum Node<A: alloc::alloc::Allocator+Copy> {
    Comment(Comment<A>),
    Document(Dom<A>),
    DocumentType(DocumentType<A>),
    Element(Element<A>),
    Text(String<A>)
}

impl<A: alloc::alloc::Allocator+Copy> Node<A> {
    pub fn as_elem(&self) -> &Element<A> {
        match *self {
            Self::Element(ref elem) => elem,
            _ => panic!("attempted to interpret a non-element node as an element")
        }
    }

    pub fn as_elem_mut(&mut self) -> &mut Element<A> {
        match *self {
            Self::Element(ref mut elem) => elem,
            _ => panic!("attempted to interpret a non-element node as an element")
        }
    }

    pub fn parent(&self) -> &Weak<RefCell<Node<A>>> {
        match *self {
            Self::Comment(_) => panic!("attempted to get the parent of a comment"),
            Self::Document(_) => panic!("attempted to get the parent of a document"),
            Self::DocumentType(_) => panic!("attempted to get the parent of a Doctype"),
            Self::Element(ref elem) => &elem.parent,
            Self::Text(_) => panic!("attempted to get the parent of a text node")
        }
    }

    pub fn parent_mut(&mut self) -> &mut Weak<RefCell<Node<A>>> {
        match *self {
            Self::Comment(_) => panic!("attempted to get the parent of a comment"),
            Self::Document(_) => panic!("attempted to get the parent of a document"),
            Self::DocumentType(_) => panic!("attempted to get the parent of a Doctype"),
            Self::Element(ref mut elem) => &mut elem.parent,
            Self::Text(_) => panic!("attempted to get the parent of a text node")
        }
    }

    pub fn children(&self) -> &Vec<Rc<RefCell<Node<A>>>, A> {
        match *self {
            Self::Comment(_) => panic!("attempted to get the children of a comment"),
            Self::Document(ref dom) => &dom.children,
            Self::DocumentType(_) => panic!("attempted to get the children of a Doctype"),
            Self::Element(ref elem) => &elem.children,
            Self::Text(_) => panic!("attempted to get the children of a text node")
        }
    }

    pub fn children_mut(&mut self) -> &mut Vec<Rc<RefCell<Node<A>>>, A> {
        match *self {
            Self::Comment(_) => panic!("attempted to get the children of a comment"),
            Self::Document(ref mut dom) => &mut dom.children,
            Self::DocumentType(_) => panic!("attempted to get the children of a Doctype"),
            Self::Element(ref mut elem) => &mut elem.children,
            Self::Text(_) => panic!("attempted to get the children of a text node")
        }
    }

    pub fn can_insert_child(&self, index: usize) -> bool {
        match *self {
            Self::Comment(_) => false,
            Self::Document(ref dom) => dom.can_insert_child(index),
            Self::DocumentType(_) => false,
            Self::Element(ref elem) => elem.can_insert_child(index),
            Self::Text(_) => false
        }
    }

    pub fn remove_child(&mut self, child: &Rc<RefCell<Node<A>>>) {
        let children = self.children_mut();
        children.remove(
            children.iter().position(|other| Rc::ptr_eq(child, other))
                .expect("attempted to remove a node that is not this node's child")
        );
    }

    // https://html.spec.whatwg.org/multipage/parsing.html#acknowledge-self-closing-flag
    pub fn acknowledge_self_closing(&mut self) {
        match *self {
            Self::Comment(_) => {},
            Self::Document(_) => {},
            Self::DocumentType(_) => {},
            Self::Element(ref mut elem) => elem.self_closing_acknowledged = true,
            Self::Text(_) => {}
        }

        // TODO: "When a start tag token is emitted with its self-closing flag set, if the flag
        // is not acknowledged when it is processed by the tree construction stage, that is a
        // non-void-html-element-start-tag-with-trailing-solidus parse error."
        // (This function isn't where we would put that logic, but we need to put it somewhere.)
    }

    // An implementation of `fmt::Display`, except that it allows an indentation to be specified with
    // the `depth` parameter.
    pub fn display(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        match *self {
            Self::Comment(ref comment) => comment.display(f, depth),
            Self::Document(ref dom) => dom.display(f, depth),
            Self::DocumentType(ref doctype) => doctype.display(f, depth),
            Self::Element(ref elem) => elem.display(f, depth),
            Self::Text(ref text) => write!(f, "{:indentation$}\"{}\"", "", text, indentation = depth * 2)
        }
    }
}

#[derive(Debug)]
pub struct Comment<A: alloc::alloc::Allocator> {
    // https://dom.spec.whatwg.org/#concept-cd-data
    pub data: String<A>,

    // https://dom.spec.whatwg.org/#concept-node-document
    // TODO: pub document: Weak<DocumentInternal<A>>
}

impl<A: alloc::alloc::Allocator+Copy> Comment<A> {
    // An implementation of `fmt::Display`, except that it allows an indentation to be specified with
    // the `depth` parameter.
    pub fn display(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        write!(f, "{:indentation$}<!--{}-->", "", self.data, indentation = depth * 2)
    }
}

#[derive(Debug)]
pub struct DocumentType<A: alloc::alloc::Allocator> {
    pub name: String<A>,
    pub public_id: String<A>,
    pub system_id: String<A>
}

impl<A: alloc::alloc::Allocator+Copy> DocumentType<A> {
    pub(super) fn from_in(doctype: tokenizer::Doctype<A>, allocator: A) -> Self {
        Self {
            name: doctype.name.unwrap_or(String::new_in(allocator)),
            public_id: doctype.public_identifier.unwrap_or(String::new_in(allocator)),
            system_id: doctype.system_identifier.unwrap_or(String::new_in(allocator))
        }
    }

    // An implementation of `fmt::Display`, except that it allows an indentation to be specified with
    // the `depth` parameter.
    pub fn display(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        write!(
            f,
            r#"{:indentation$}[DocumentType name="{}" publicId="{}" systemId="{}"]"#,
            "",
            self.name,
            self.public_id,
            self.system_id,
            indentation = depth * 2
        )
    }
}

