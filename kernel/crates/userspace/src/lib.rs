/* Copyright (c) 2021-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#![feature(maybe_uninit_slice)]

use {
    core::{
        convert::TryInto,
        mem::MaybeUninit,
    },
    io::{Read, Seek, SeekFrom},
    memory::{
        phys::RegionType,
        virt::paging::{self, RootPageTable},
    },
};

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
pub struct UserspaceStr<'a, E: Read+Seek+Clone> {
    root_page_table: &'a RootPageTable,
    exe_reader: E,
    start_addr_userspace: usize,
    len: usize,
    start_ptr_kernel: *const u8,
    page_size: usize // Cached to avoid frequently polling an unchanging atomic variable
}

impl<'a, E: Read+Seek+Clone> UserspaceStr<'a, E> {
    /// Creates a new view into a userspace string.
    ///
    /// # Returns
    /// The userspace string, or `None` if part of the string is missing from virtual memory.
    pub fn from_raw_parts(
        root_page_table: &'a RootPageTable,
        exe_reader: E,
        start_addr_userspace: usize,
        len: usize,
    ) -> Option<Self> {
        // FIXME: Mark the pages that contain the string as unswappable until this object is dropped.
        Some(Self {
            root_page_table,
            exe_reader: exe_reader.clone(),
            start_addr_userspace,
            len,
            start_ptr_kernel: userspace_addr_to_kernel_addr(
                start_addr_userspace,
                root_page_table,
                exe_reader,
            )? as *const u8,
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
    pub fn match_and_advance<P: AsRef<[u8]>+?Sized>(&self, prefix: &P) -> Option<Self> {
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

    /// Returns the first byte of this string.
    ///
    /// # Panics
    /// If the given string is empty.
    pub fn head(&self) -> u8 {
        assert!(self.len > 0);
        unsafe { *self.start_ptr_kernel }
    }

    /// Returns this string with its first byte removed.
    ///
    /// # Panics
    /// If the given string is empty.
    pub fn tail(self) -> Self {
        assert!(self.len > 0);
        let start_addr_userspace = self.start_addr_userspace.wrapping_add(1);
        let mut next_kernel_addr = (self.start_ptr_kernel as usize).wrapping_add(1);
        if next_kernel_addr % self.page_size == 0 {
            next_kernel_addr = userspace_addr_to_kernel_addr(
                start_addr_userspace,
                self.root_page_table,
                self.exe_reader.clone(),
            ).unwrap();
        }
        Self {
            root_page_table: self.root_page_table,
            exe_reader: self.exe_reader,
            start_addr_userspace,
            len: self.len - 1,
            start_ptr_kernel: next_kernel_addr as *const u8,
            page_size: self.page_size
        }
    }
}

fn userspace_addr_to_kernel_addr<E: Read+Seek>(
    userspace_addr: usize,
    root_page_table: &RootPageTable,
    mut exe_reader: E,
) -> Option<usize> {
    root_page_table.userspace_addr_to_kernel_addr(
        userspace_addr,
        RegionType::Ram,
        |addr, mut buffer| {
            exe_reader.seek(SeekFrom::Start(addr.try_into().map_err(|_| ())?)).map_err(|_| ())?;
            while buffer.len() > 0 {
                match exe_reader.read(unsafe { MaybeUninit::slice_assume_init_mut(buffer) }) {
                    Ok(0) => break, // EOF
                    Ok(n) => buffer = &mut buffer[n .. ],
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => return Err(()),
                };
            }
            buffer.fill(MaybeUninit::new(0)); // In case of EOF
            Ok(())
        }
    )
}

impl<'a, E: Read+Seek+Clone> Read for UserspaceStr<'a, E> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = usize::min(self.len(), buf.len());
        for i in 0 .. count {
            buf[i] = self.head();
            *self = self.clone().tail();
        }
        Ok(count)
    }
}
