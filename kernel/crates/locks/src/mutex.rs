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

//! This module defines a mutex, but without any provision for blocking. That is by design: the
//! calling code should be able to find something else to do with those CPU cycles.

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering}
};

/// A lock around a value that requires mutual exclusion.
#[derive(Debug)]
pub struct Mutex<T> {
    value: UnsafeCell<T>,
    locked: AtomicBool
}

/// An RAII guard for a `Mutex`. This type coerces to `&mut T` and unlocks the mutex when dropped.
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>
}

impl<T> Mutex<T> {
    /// Constructs a new mutex around the given value.
    pub const fn new(value: T) -> Mutex<T> {
        Mutex {
            value: UnsafeCell::new(value),
            locked: AtomicBool::new(false)
        }
    }

    /// Attempts to lock the mutex.
    ///
    /// # Returns
    /// An RAII guard if successful, else `Err(())`.
    pub fn try_lock(&self) -> Result<MutexGuard<T>, ()> {
        match self.locked.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Ok(MutexGuard { mutex: self }),
            Err(_) => Err(())
        }
    }
}

unsafe impl<T> Sync for Mutex<T> {}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}
