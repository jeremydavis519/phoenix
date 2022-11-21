/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate is the kernel's memory manager. It is platform-independent and provides enough
//! abstraction to allow Rust's `alloc` crate to run on top of it. Therefore, the rest of the
//! kernel should interact with this crate in only two ways:
//! 1. by providing a memory map so the memory manager knows what memory it can use and
//! 2. through the provided allocator.
//! For the most part, though, the rest of the kernel can simply use the standard `alloc` crate to
//! deal with dynamic memory.

#![cfg_attr(not(test), no_std)]

#![feature(allocator_api, alloc_error_handler)]
#![feature(const_type_name)]
#![feature(inline_const)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(panic_info_message)]
#![feature(slice_ptr_get)]

#![deny(warnings, missing_docs)]

extern crate alloc;
#[cfg(not(feature = "unit-test"))] #[macro_use] extern crate bitflags;
#[macro_use] extern crate shared;

#[macro_use] extern crate macros_unreachable;

use {
    alloc::alloc::{GlobalAlloc, Allocator, Layout},
    core::ptr::{self, NonNull}
};
#[cfg(not(feature = "unit-test"))]
use i18n::Text;

pub mod phys;
pub mod virt;

pub mod allocator;

/// The kernel's global allocator. This should not be used directly except when Rust's abstractions
/// like `Box` and `Vec` don't work for some reason--e.g. when we need a block of memory at a
/// certain physical address.
#[cfg(not(feature = "unit-test"))]
static ALLOCATOR: allocator::AllMemAlloc = allocator::AllMemAlloc;

// We need a wrapper because Rust doesn't let the rest of the program access the global allocator.
// TODO: Do we? This was true when we stored it in a static variable, but accessing it by its
// type's name should work (since it's a ZST).
struct AllocWrapper {
    allocator: &'static allocator::AllMemAlloc
}

unsafe impl GlobalAlloc for AllocWrapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocator.allocate(layout)
            .map(|nonnull| nonnull.as_mut_ptr() as *mut u8)
            .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(ptr) = NonNull::new(ptr) {
            self.allocator.deallocate(ptr, layout);
        }
    }
}

#[cfg(not(feature = "unit-test"))]
#[global_allocator]
static ALLOC_WRAPPER: AllocWrapper = AllocWrapper { allocator: &ALLOCATOR };

#[cfg(not(feature = "unit-test"))]
#[alloc_error_handler]
fn out_of_memory(layout: Layout) -> ! {
    // This is called after a failed allocation. It should never happen unless there really
    // is too little RAM on the computer to run the kernel.
    panic!("{}", Text::OutOfMemory(layout.size(), layout.align()));
}
