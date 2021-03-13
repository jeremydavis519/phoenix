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

//! This module provides ways to define a static or otherwise shared variable that should be
//! initialized later and never changed again. These implementations may not be thread-safe (the
//! thread-unsafe parts are marked as `unsafe`) because they are both lockless (no mutexes) and
//! generic (no atomic values).

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// This struct represents a piece of data whose value should only ever be set once. It behaves just
/// like the `Once` from the `spin` crate except that it doesn't use a spinlock. As a result, it
/// lacks protection from simultaneous writes. Its use requires an `unsafe` block and should ensure
/// that no two threads could attempt to initialize it at the same time with different values.
///
/// The motivation for making this struct without synchronization primitives is so that if one core
/// stops in the middle of the initialization to run a hypervisor, a second core will be able to do
/// the initialization itself instead of blocking, which should improve the boot time in such systems.
#[derive(Debug)]
pub struct Once<T> {
    // Holds the return value of `call_once`, or `None` if it hasn't returned yet. We use an `Option` here
    // instead of `mem::uninitialized` in order to guarantee that overwritten values are dropped.
    value: UnsafeCell<Option<T>>,

    // We use a separate AtomicBool instead of just making `value` an `Option` because we can't guarantee
    // that `Option<T>` will change from `None` to `Some` atomically.
    finished: AtomicBool
}

unsafe impl<T: Send> Send for Once<T> {}
unsafe impl<T: Sync> Sync for Once<T> {}

impl<T> Once<T> {
    /// Creates a new `Once` value.
    pub const fn new() -> Once<T> {
        Once {
            value: UnsafeCell::new(None),
            finished: AtomicBool::new(false)
        }
    }

    /// Performs an initialization routine once. The given closure will be executed if `call_once` has
    /// never finished. If it has finished before, the closure will *not* be executed.
    ///
    /// This method will *not* block the calling thread if another initialization routine is already
    /// running. Instead, both will run in parallel with no protection from concurrent writes.
    ///
    /// When this method returns, it is guaranteed that some initialization has run and completed (it
    /// may or may not be the closure specified). The returned reference will point to the result from
    /// the closure that was run.
    ///
    /// # Safety
    /// This method is marked as `unsafe` because it allows data races. If it is possible for two
    /// threads to call this method before either of them has finished the initialization, then the
    /// calling code must guarantee that both initializations will result in exactly the same value
    /// and that it will never be changed until *all* initializations are complete. Otherwise, this
    /// function's behavior is undefined, since it returns a reference to a value in an undefined
    /// state.
    pub unsafe fn call_once<F>(&self, builder: F) -> &T
        where F: FnOnce() -> T
    {
        if self.finished.load(Ordering::Acquire) {
            self.force_get() // Value has already been created
        } else {
            // Save the value for later and then return it.
            *self.value.get() = Some(builder());
            self.finished.store(true, Ordering::Release);
            self.force_get()
        }
    }

    /// Returns a pointer to the calculated value iff the `Once` has already been initialized.
    pub fn try_get(&self) -> Option<&T> {
        if self.finished.load(Ordering::Acquire) {
            unsafe { Some(self.force_get()) }
        } else {
            None
        }
    }

    // Returns `&v`, where `self.value` constains `Some(v)`. This can produce undefined behavior if
    // `self.value` contains `None`.
    unsafe fn force_get(&self) -> &T {
        match &*self.value.get() {
            None => unreachable_debug!(
                "This function is private and is only called after a value has been created."
            ),
            Some(v) => &v
        }
    }
}

/// This macro makes a wrapper for the `Once` struct that can be given an initializer expression
/// when it's defined instead of when it's used. It fills the role of the `lazy_static` crate
/// without requiring either `std` or the `spin` crate.
///
/// Use of this macro requires an `unsafe` block. It's part of the definition. The reason is that
/// we can't prevent data races and the macro can't guarantee that the values given in the
/// initializers are actually constant.
#[macro_export]
macro_rules! lazy_static {
    ($(unsafe { $($(#[$attr:meta])* $vis:vis static ref $var:ident : $type:ty = $initializer:expr ;)* })*) => {
        $(
            $(
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                $vis struct $var {
                    once: $crate::once::Once<$type>
                }
                impl ::core::ops::Deref for $var {
                    type Target = $type;
                    fn deref(&self) -> &Self::Target {
                        unsafe { self.once.call_once(|| $initializer) }
                    }
                }
                $(#[$attr])*
                $vis static $var: $var = $var { once: $crate::once::Once::new() };
            )*
        )*
    }
}
