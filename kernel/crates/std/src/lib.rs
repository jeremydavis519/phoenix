/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate implements a subset of the Rust standard library for the kernel's use. The interface
//! is kept as similar as possible, although many unused parts are omitted, and parts of the actual
//! standard library should be used whenever possible instead of duplicating them here.
// TODO: Move all of this stuff to other crates. Using the `std` name turns out to be more awkward
// than I expected.

#![no_std]

#![feature(allocator_api)]
#![feature(lang_items)]

#![deny(warnings, missing_docs)]

extern crate alloc;
#[cfg_attr(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"), macro_use)]
    extern crate io as io_impl;

pub mod fmt;
pub mod panic;
