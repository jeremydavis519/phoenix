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

//! This module defines all the types necessary to describe a node tree, as defined by the DOM
//! standard.

mod attr;
mod cdata_section;
mod character_data;
mod comment;
mod document;
mod document_fragment;
mod document_type;
mod element;
mod named_node_map;
mod processing_instruction;
mod text;

pub use self::{
    attr::*,
    cdata_section::*,
    comment::*,
    document::*,
    document_fragment::*,
    document_type::*,
    element::*,
    named_node_map::*,
    processing_instruction::*,
    text::*
};

use {
    alloc::{
        boxed::Box,
        rc::Rc,
        vec::Vec,
    },
    core::{
        convert::TryFrom,
        ptr,
    },
    intertrait::cast::CastRc,
    crate::idl::{self, DomString},
};

extern "Rust" {
    // https://dom.spec.whatwg.org/#concept-node-clone-ext
    fn do_cloning_steps(
        copy: Rc<dyn idl::Node>,
        node: Rc<dyn idl::Node>,
        document: Rc<dyn idl::Document>,
        clone_children: bool
    );
}

impl idl::NodeList for Vec<Rc<dyn idl::Node>> {
    // https://dom.spec.whatwg.org/#dom-nodelist-item
    fn item(self: Rc<Self>, index: u32) -> Option<Rc<dyn idl::Node>> {
        self.get(usize::try_from(index).expect("NodeList index overflow"))
            .map(|rc| rc.clone())
    }

    // https://dom.spec.whatwg.org/#dom-nodelist-length
    fn length(self: Rc<Self>) -> u32 {
        u32::try_from(self.len()).expect("NodeList length overflow")
    }

    fn _iter<'a>(self: Rc<Self>) -> Box<dyn Iterator<Item = Rc<dyn idl::Node>> + 'a> where Self: 'a {
        struct It {
            nodes: Rc<Vec<Rc<dyn idl::Node>>>,
            idx: usize
        }
        impl Iterator for It {
            type Item = Rc<dyn idl::Node>;
            fn next(&mut self) -> Option<Self::Item> {
                let i = self.idx;
                if i < self.nodes.len() {
                    self.idx += 1;
                    Some(self.nodes[i].clone())
                } else {
                    None
                }
            }
        }

        Box::new(It { nodes: self, idx: 0 })
    }
}

// https://dom.spec.whatwg.org/#concept-descendant-text-content
fn descendant_text_content<N: idl::Node + ?Sized>(node: &Rc<N>) -> DomString {
    let mut text = DomString::new();
    for child in node.clone().childNodes()._iter() {
        if let Ok(text_node) = child.clone().cast::<dyn idl::Text>() {
            text.extend(text_node.data().iter());
        } else {
            text.extend(descendant_text_content(&child).iter());
        }
    }
    text
}

// https://dom.spec.whatwg.org/#dom-node-previoussibling
fn previous_sibling<N: idl::Node + ?Sized>(node: &Rc<N>) -> Option<Rc<dyn idl::Node>> {
    let siblings = node.clone().parentNode()?.childNodes();

    for i in 1 .. siblings.clone().length() {
        if ptr::eq(&**node as *const N as *const u8,
                &*siblings.clone().item(i).unwrap() as *const dyn idl::Node as *const u8) {
            return siblings.item(i - 1).clone();
        }
    }
    None
}

// https://dom.spec.whatwg.org/#dom-node-nextsibling
fn next_sibling<N: idl::Node + ?Sized>(node: &Rc<N>) -> Option<Rc<dyn idl::Node>> {
    let siblings = node.clone().parentNode()?.childNodes();

    for i in 0 .. siblings.clone().length() - 1 {
        if ptr::eq(&**node as *const N as *const u8,
                &*siblings.clone().item(i).unwrap() as *const dyn idl::Node as *const u8) {
            return siblings.item(i + 1).clone();
        }
    }
    None
}

// https://dom.spec.whatwg.org/#concept-node-pre-insert
fn pre_insert<N, P, C>(node: Rc<N>, parent: Rc<P>, next_child: Rc<C>) -> Rc<N>
        where N: idl::Node + ?Sized,
              P: idl::Node + ?Sized,
              C: idl::Node + ?Sized, {
    // 1. Ensure pre-insertion validity of node into parent before child.

    // 2. Let referenceChild be child.
    let mut reference_child = Some(next_child.clone());

    // 3. If referenceChild is node, then set referenceChild to node’s next sibling.
    if ptr::eq(&*next_child, &*node) {
        reference_child = node.nextSibling();
    }

    // 4. Insert node into parent before referenceChild.
    insert_before(&node, &parent, reference_child);

    // 5. Return node.
    node
}

// https://dom.spec.whatwg.org/#concept-node-insert
fn insert_before<N, P, C>(node: &Rc<N>, parent: &Rc<P>, next_child: Option<Rc<C>>)
        where N: idl::Node + ?Sized,
              P: idl::Node + ?Sized,
              C: idl::Node + ?Sized, {
    // 1. Let nodes be node’s children, if node is a DocumentFragment node; otherwise « node ».
    let doc_fragment = node.clone().cast::<dyn idl::DocumentFragment>();
    let nodes = if let Ok(ref node) = doc_fragment {
        node.clone().childNodes()
    } else {
        Rc::new(vec![node.clone()])
    };

    // 2. Let count be node's size.
    let count = nodes.length();

    // 3. If count is 0, then return.
    if count == 0 { return; }

    // 4. If node is a DocumentFragment node, then:
    if let Ok(node) = doc_fragment {
        // 1. Remove its children with the suppress observers flag set.
        todo!();

        // 2. Queue a tree mutation record for node with « », nodes, null, and null.
        todo!();
    }

    // 5. If child is non-null, then:
    if let Some(ref child) = next_child {
        // 1. For each live range whose start node is parent and start offset is greater than child’s index, ...
        /* TODO: for ... */ {
            // ... increase its start offset by count.
            // TODO
        }

        // 2. For each live range whose end node is parent and end offset is greater than child’s index, ...
        /* TODO: for ... */ {
            // ... increase its end offset by count.
            // TODO
        }
    }

    // 6. Let previousSibling be child’s previous sibling or parent’s last child if child is null.
    let previousSibling = match next_child {
        Some(ref child) => child.clone().previousSibling(),
        None => parent.clone().lastChild(),
    };

    // 7. For each node in nodes, in tree order:
    for node in nodes._iter() {
        // 1. Adopt node into parent’s node document.
        // TODO

        // 2. If child is null, then append node to parent’s children.
        if next_child.is_none() {
            parent.append(node);
        }
        
        // 3. Otherwise, insert node into parent’s children before child’s index.
        else {
            parent.clone().insert_at_index(node, parent.clone().index_of(next_child.clone()));
        }

        // 4. If parent is a shadow host whose shadow root’s slot assignment is "named" and node is a slottable,
        //    then assign a slot for node.
        if let Ok(parent) = parent.clone().cast::<dyn idl::Element>() {
            if let Some(root) = parent.shadowRoot() {
                if root.slotAssignment() == "named" /* TODO: && node.is_slottable() */ {
                    // TODO
                    todo!();
                }
            }
        }

        // 5. If parent’s root is a shadow root, and parent is a slot whose assigned nodes is the empty list,
        //    then run signal a slot change for parent.
        if let Ok(root) = parent.getRootNode().cast::<dyn idl::ShadowRoot>() {
            // TODO
            todo!();
        }

        // 6. Run assign slottables for a tree with node’s root.
        // TODO

        // 7. For each shadow-including inclusive descendant inclusiveDescendant of node, in shadow-including tree order:
        // TODO
    }
        
    // 8. If suppress observers flag is unset, then queue a tree mutation record for parent with nodes, « »,
    //    previousSibling, and child.
    // TODO

    // 9. Run the children changed steps for parent.
    // TODO
}
