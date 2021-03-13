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

//! This module defines the [`Future`](struct.Future.html) type, which represents a value that the
//! kernel has promised to give us but may not have provided yet. This type is a core part of the
//! Phoenix system call model, since almost every system call is asynchronous.

use {
    core::{
        fmt,
        ptr,
        sync::atomic::{AtomicBool, Ordering}
    },
    super::syscall
};

/// A promised but possibly not-yet-available return value from a system call.
#[derive(Debug)]
pub struct Future<'a, T> {
    pub(crate) promised: &'a mut PromisedValue<T>
}

impl<'a, T> Future<'a, T> {
    /// Checks whether this future has a value yet and returns a reference to it if so.
    pub fn poll(&self) -> Option<&T> {
        if self.promised.exists.load(Ordering::Acquire) {
            Some(&self.promised.value)
        } else {
            None
        }
    }

    /// Checks whether this future has a value yet and returns a mutable reference to it if so.
    pub fn poll_mut(&mut self) -> Option<&mut T> {
        if self.promised.exists.load(Ordering::Acquire) {
            Some(&mut self.promised.value)
        } else {
            None
        }
    }

    /// Moves the inner value out of this future.
    ///
    /// # Returns
    /// The inner value, or an error if the future does not yet have a value.
    pub fn into_inner(self) -> Result<T, FutureUnwrapError> {
        if self.promised.exists.swap(false, Ordering::AcqRel) {
            Ok(unsafe { ptr::read(&self.promised.value as *const T) })
        } else {
            Err(FutureUnwrapError)
        }
    }

    /// Transfers control to the kernel until this future has a value, then returns that value.
    pub fn block(self) -> T {
        syscall::future_block(self.promised as *mut _ as usize);
        self.into_inner().unwrap()
    }
}

impl<'a, T> Drop for Future<'a, T> {
    fn drop(&mut self) {
        // TODO: Tell the kernel it can free this future.
        unimplemented!("Future::drop");
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct PromisedValue<T> {
    exists: AtomicBool,
    value:  T
}

/// An error resulting from trying to unwrap a `Future` that does not yet have a value.
#[derive(Debug)]
pub struct FutureUnwrapError;

impl fmt::Display for FutureUnwrapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "attempted to unwrap a future that did not have a value yet")
    }
}
