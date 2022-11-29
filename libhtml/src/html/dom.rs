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

//! This module defines the DOM tree contained in a document.

use {
    alloc::{
        rc::Rc,
        vec::Vec
    },
    core::{
        cell::RefCell,
        fmt
    },
    super::node::Node
};

/// The DOM. This object stores all the contents of a document in a tree structure.
#[derive(Debug)]
pub struct Dom<A: alloc::alloc::Allocator+Copy> {
    pub(super) children: Vec<Rc<RefCell<Node<A>>>, A>
}

impl<A: alloc::alloc::Allocator+Copy> Dom<A> {
    /// Constructs a new DOM with the given allocator.
    pub fn new(allocator: A) -> Self {
        Self {
            children: Vec::new_in(allocator)
        }
    }

    pub fn can_insert_child(&self, index: usize) -> bool {
        // FIXME: This shouldn't always be true, but I haven't found where in the spec it's spelled out yet.
        true
    }

    // An implementation of `fmt::Display`, except that it allows an indentation to be specified with
    // the `depth` parameter.
    pub fn display(&self, f: &mut fmt::Formatter, depth: usize) -> fmt::Result {
        write!(f, "[Document]")?;
        
        for node in self.children.iter() {
            writeln!(f)?;
            node.borrow().display(f, depth + 1)?;
        }
        Ok(())
    }
}

impl<A: alloc::alloc::Allocator+Copy> fmt::Display for Dom<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(f, 0)
    }
}
