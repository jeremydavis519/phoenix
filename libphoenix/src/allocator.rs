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

//! This crate defines the standard allocator for Phoenix programs.
//!
//! This sets the global allocator, since most programs shouldn't need to define their own
//! allocators, but it's not required. If a program does not want the global allocator, it should
//! disable the `global-allocator` feature (and every library that depends on `libphoenix` should
//! expose that feature in its own `Cargo.toml`, as follows:
//!
//! ```toml
//! [features]
//! global-allocator = ["libphoenix/global-allocator"]
//! ```

// FIXME: Write a more flexible allocator that uses system calls to get new pages, and use it
//        instead of this extremely limited bump allocator.

use {
    alloc::alloc::{Layout, GlobalAlloc},
    core::{
        cell::UnsafeCell,
        ptr,
        sync::atomic::{AtomicUsize, Ordering}
    }
};

#[cfg(feature = "global-allocator")]
#[global_allocator]
static ALLOCATOR: BumpAlloc<0x100000> = BumpAlloc::new();

/// A simple allocator that allocates from a fixed-size buffer and can never deallocate.
///
/// A bump allocator is useful for when you need to allocate finitely many objects on the heap or
/// failure to allocate is acceptable and when allocation speed is crucial. There is no faster type
/// of allocator, unless one counts static allocation.
#[derive(Debug)]
pub struct BumpAlloc<const SIZE: usize> {
    memory: [UnsafeCell<u8>; SIZE],
    cursor: AtomicUsize
}

impl<const SIZE: usize> BumpAlloc<SIZE> {
    /// Makes a new bump allocator.
    pub const fn new() -> Self {
        const ZERO: UnsafeCell<u8> = UnsafeCell::new(0);
        BumpAlloc {
            memory: [ZERO; SIZE],
            cursor: AtomicUsize::new(0)
        }
    }
}

unsafe impl<const SIZE: usize> GlobalAlloc for BumpAlloc<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut old_cursor = self.cursor.load(Ordering::Acquire);
        loop {
            let align = layout.align();
            let aligned_cursor = old_cursor.wrapping_add(align).wrapping_sub(1) / align * align;
            if aligned_cursor < old_cursor {
                // The requested alignment was so big that the new cursor overflowed a `usize`!
                return ptr::null_mut();
            }
            if let Some(new_cursor) = aligned_cursor.checked_add(layout.size()) {
                if new_cursor < self.memory.len() {
                    // There is enough room, so update the cursor.
                    match self.cursor.compare_exchange(old_cursor, new_cursor, Ordering::AcqRel, Ordering::Acquire) {
                        Ok(_) => return self.memory[aligned_cursor].get(),
                        Err(x) => old_cursor = x
                    };
                } else {
                    // Too little memory left.
                    return ptr::null_mut();
                }
            } else {
                // The requested size would overflow the entire memory space!
                return ptr::null_mut();
            }
        }
    }
    
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}

unsafe impl<const SIZE: usize> Sync for BumpAlloc<SIZE> {}
