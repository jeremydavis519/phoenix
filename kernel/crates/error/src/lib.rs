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

//! An extremely barebones implementation of some of std::error from the Rust standard library.

#![no_std]

#![deny(warnings, missing_docs)]
#![feature(allocator_api)]

extern crate alloc;

use {
    alloc::{
        alloc::AllocError,
        string::FromUtf8Error
    },
    core::fmt::{Debug, Display}
};

/// Base functionality for all errors in Rust.
pub trait Error: Debug + Display {
    // The `description` and `cause` functions are both deprecated in the Rust standard library.

    /// Indicates the error that led to this one, if any.
    fn source(&self) -> Option<&'static dyn Error> {
        None
    }

    // TODO: Add the `backtrace` method and the `Backtrace` type.
}

impl Error for AllocError {}
impl Error for FromUtf8Error {}
