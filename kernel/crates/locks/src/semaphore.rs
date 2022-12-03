/* Copyright (c) 2021-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines a semaphore, which allows a certain number of threads in at a time. If that
//! number is set to 1, this is equivalent to a mutex.

use core::{
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering}
};

/// A semaphore around a value that should be accessed by only a limited number of visitors at a
/// time. The maximum number of visitors is specified during construction.
pub struct Semaphore<T> {
    value: T,
    tickets: AtomicUsize
}

/// An RAII guard for a `Semaphore`. This type coerces to `&T` and closes the semaphore access when
/// dropped. It cannot be converted to `&mut T` because a semaphore does not guarantee mutually
/// exclusive access in general.
pub struct SemaphoreGuard<'a, T> {
    semaphore: &'a Semaphore<T>
}

impl<T> Semaphore<T> {
    /// Constructs a new semaphore around the given value.
    pub const fn new(value: T, max_visitors: usize) -> Semaphore<T> {
        Semaphore {
            value,
            tickets: AtomicUsize::new(max_visitors)
        }
    }

    /// Attempts to access the contained value, retrying as long as the number of visitors has not
    /// reached the maximum.
    ///
    /// # Returns
    /// An RAII guard if successful, else `Err(())`.
    pub fn try_access(&self) -> Result<SemaphoreGuard<T>, ()> {
        let mut tickets = self.tickets.load(Ordering::Acquire);
        while tickets > 0 {
            match self.tickets.compare_exchange_weak(tickets, tickets - 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Ok(SemaphoreGuard { semaphore: self }),
                Err(x) => tickets = x
            };
        }
        Err(())
    }

    /// Attempts to access the contained value, trying only once. This can produce spurrious
    /// failures, but it runs in constant time.
    ///
    /// # Returns
    /// An RAII guard if successful, else `Err(x)`, where `x` is how many more visitors can fit in
    /// the semaphore right now.
    pub fn try_access_weak(&self) -> Result<SemaphoreGuard<T>, usize> {
        let tickets = self.tickets.load(Ordering::Acquire);
        if tickets > 0 {
            match self.tickets.compare_exchange_weak(tickets, tickets - 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Ok(SemaphoreGuard { semaphore: self }),
                Err(x) => return Err(x)
            };
        }
        Err(0)
    }

    /// Returns a reference to the contained value without incrementing the number of visitors.
    ///
    /// # Safety
    /// This is extremely unsafe because it breaks the semaphore's usual guarantee. It must be used
    /// only in cases where it can be proven that the guarantee of limited access is unnecessary.
    pub const unsafe fn force_access(&self) -> &T {
        &self.value
    }
}

impl<'a, T> Deref for SemaphoreGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.semaphore.value
    }
}

impl<'a, T> Drop for SemaphoreGuard<'a, T> {
    fn drop(&mut self) {
        self.semaphore.tickets.fetch_add(1, Ordering::AcqRel);
    }
}
