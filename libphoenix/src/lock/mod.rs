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

//! This module defines some standard types of locks for concurrent programs.
//!
//! The API is modeled after `std::sync` in the Rust standard library, but with changes wherever a
//! direct mapping wouldn't make sense.

mod rw_lock;
pub use rw_lock::*;

use core::fmt;

/// The result of a nonblocking locking method.
pub type TryLockResult<Guard> = Result<Guard, TryLockError>;

/// An error that can occur while trying to acquire a lock with a nonblocking method.
#[derive(Debug)]
pub enum TryLockError {
    /// The locking operation would have needed to block.
    WouldBlock,
}

impl fmt::Display for TryLockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::WouldBlock => write!(f, "would block"),
        }
    }
}
