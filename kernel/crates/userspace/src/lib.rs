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

//! This crate provides the kernel with structured ways to access userspace. This is crucial when
//! accepting a string as an argument to a system call, for instance.

#![no_std]
#![deny(warnings, missing_docs)]

use memory::virt::paging::{self, RootPageTable};

/// A borrowed string that is stored in userspace.
///
/// This type is conceptually similar to `&str`, but there are a few important differences:
/// * While no well-behaved program will have one thread modify a string that another thread is
///   currently using as an argument to a system call, we have to assume that not every program will
///   be well-behaved. As such, it *must never be assumed* that two bytes read from the string at
///   the same index but at different times will be identical.
/// * Because we can't make that assumption, any parsing of the string has to be done one byte at a
///   time. An API is provided to facilitate that. (Trying to get around this by declaring undefined
///   behavior would be a nightmare from a security perspective.)
#[derive(Debug, Clone)]
pub struct UserspaceStr<'a> {
    root_page_table: &'a RootPageTable,
    start_addr_userspace: usize,
    len: usize,
    start_ptr_kernel: *const u8,
    page_size: usize // Cached to avoid frequently polling an unchanging atomic variable
}

impl<'a> UserspaceStr<'a> {
    /// Creates a new view into a userspace string.
    ///
    /// # Returns
    /// The userspace string, or `None` if part of the string is missing from virtual memory.
    pub fn from_raw_parts(root_page_table: &'a RootPageTable, start_addr_userspace: usize, len: usize)
            -> Option<UserspaceStr<'a>> {
        // FIXME: Mark the pages that contain the string as unswappable until this object is dropped.
        Some(UserspaceStr {
            root_page_table,
            start_addr_userspace,
            len,
            start_ptr_kernel: root_page_table.userspace_addr_to_kernel_addr(start_addr_userspace)? as *const u8,
            page_size: paging::page_size()
        })
    }

    /// Returns the length of the string.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Determines whether the string is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Looks for a match at the beginning of the string, then advances past it.
    ///
    /// This function is particularly useful for a recursive-descent parser, since it leaves the
    /// original `UserspaceStr` unchanged and returns a copy that is past the match.
    ///
    /// # Returns
    /// The part of this string that remains after removing the prefix, or `None` if the userspace
    /// string doesn't start with that prefix.
    pub fn match_and_advance<P: AsRef<[u8]>+?Sized>(&self, prefix: &P) -> Option<UserspaceStr<'a>> {
        let mut prefix = prefix.as_ref();
        let mut rest = self.clone();
        loop {
            if prefix.len() == 0 {
                return Some(rest);
            }
            if rest.len() == 0 {
                return None;
            }
            if rest.head() != prefix[0] {
                return None;
            }

            prefix = &prefix[1 .. ];
            rest = rest.tail();
        }
    }

    fn head(&self) -> u8 {
        assert!(self.len > 0);
        unsafe { *self.start_ptr_kernel }
    }

    fn tail(&self) -> UserspaceStr<'a> {
        assert!(self.len > 0);
        let start_addr_userspace = self.start_addr_userspace.wrapping_add(1);
        let mut next_kernel_addr = (self.start_ptr_kernel as usize).wrapping_add(1);
        if next_kernel_addr % self.page_size == 0 {
            next_kernel_addr = self.root_page_table.userspace_addr_to_kernel_addr(start_addr_userspace).unwrap();
        }
        UserspaceStr {
            root_page_table: self.root_page_table,
            start_addr_userspace,
            len: self.len - 1,
            start_ptr_kernel: next_kernel_addr as *const u8,
            page_size: self.page_size
        }
    }
}
