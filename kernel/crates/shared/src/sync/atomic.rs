/* Copyright (c) 2019-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines some new atomic types that are not already defined in `core::sync::atomic`.

use core::convert::{TryFrom, TryInto};
use core::fmt::Debug;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

/// A value that can be converted both to and from a `u64`, stored atomically. This type adds type
/// safety to such use cases as atomically accessing a set of flags, as defined by the `bitflags`
/// crate. Note that the appropriate traits will have to be implemented in this case, since
/// `bitflags` doesn't use `TryFrom` and `Into`.
#[derive(Debug)]
pub struct Atomic64Bit<T: ?Sized+TryFrom<u64>+Into<u64>+Debug>
        where <T as TryFrom<u64>>::Error: Debug {
    internal: AtomicU64,
    _phantom: PhantomData<T>
}

impl<T: ?Sized+TryFrom<u64>+Into<u64>+Debug> Atomic64Bit<T>
        where <T as TryFrom<u64>>::Error: Debug {
    /// Makes a new `Atomic64Bit` from the given non-atomic value.
    pub fn new(v: T) -> Atomic64Bit<T> {
        Atomic64Bit {
            internal: AtomicU64::new(v.into()),
            _phantom: PhantomData
        }
    }

    /// Makes a new `Atomic64Bit` from the given non-atomic value.
    ///
    /// # Safety
    /// The given value must be a valid result of calling `Into::into` on a value of type `T`.
    pub const unsafe fn new_raw(v: u64) -> Atomic64Bit<T> {
        Atomic64Bit {
            internal: AtomicU64::new(v),
            _phantom: PhantomData
        }
    }

    /// Converts the given owned atomic value into a non-atomic value.
    pub fn into_inner(self) -> T {
        self.internal.into_inner().try_into().unwrap()
    }

    /// Atomically loads the current value.
    pub fn load(&self, order: Ordering) -> T {
        self.internal.load(order).try_into().unwrap()
    }

    /// Atomically stores the given value.
    pub fn store(&self, val: T, order: Ordering) {
        self.internal.store(val.into(), order)
    }

    /// Atomically swaps the given value with the current value.
    pub fn swap(&self, val: T, order: Ordering) -> T {
        self.internal.swap(val.into(), order).try_into().unwrap()
    }

    /// Performs an atomic compare-and-exchange operation.
    pub fn compare_exchange(&self, current: T, new: T, success: Ordering, failure: Ordering) -> Result<T, T> {
        self.internal.compare_exchange(current.into(), new.into(), success, failure)
            .map(|x| x.try_into().unwrap())
            .map_err(|x| x.try_into().unwrap())
    }

    /// Performs an atomic compare-and-exchange operation that is allowed to fail spurriously.
    pub fn compare_exchange_weak(&self, current: T, new: T, success: Ordering, failure: Ordering) -> Result<T, T> {
        self.internal.compare_exchange_weak(current.into(), new.into(), success, failure)
            .map(|x| x.try_into().unwrap())
            .map_err(|x| x.try_into().unwrap())
    }

    /*pub fn fetch_add(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_add(val.into(), order).try_into().unwrap()
    }

    pub fn fetch_sub(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_sub(val.into(), order).try_into().unwrap()
    }

    pub fn fetch_and(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_and(val.into(), order).try_into().unwrap()
    }

    pub fn fetch_nand(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_nand(val.into(), order).try_into().unwrap()
    }*/

    /// Atomically fetches the current value and updates it with a bitwise OR operation against the
    /// given value.
    pub fn fetch_or(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_or(val.into(), order).try_into().unwrap()
    }

    /*pub fn fetch_xor(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_xor(val.into(), order).try_into().unwrap()
    }*/

    /// Atomically fetches the current value, applies the given function to it, and atomically sets
    /// it to the return value.
    pub fn fetch_update<F>(&self, set_order: Ordering, fetch_order: Ordering, mut f: F) -> Result<T, T>
            where F: FnMut(T) -> Option<T> {
        self.internal.fetch_update(set_order, fetch_order, |x| f(x.try_into().unwrap()).map(|x| x.into()))
            .map(|x| x.try_into().unwrap())
            .map_err(|x| x.try_into().unwrap())
    }

    /*pub fn fetch_max(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_max(val.into(), order).try_into().unwrap()
    }

    pub fn fetch_min(&self, val: T, order: Ordering) -> T {
        self.internal.fetch_min(val.into(), order).try_into().unwrap()
    }*/
}

impl<T: ?Sized+TryFrom<u64>+Into<u64>+Debug+Default> Default for Atomic64Bit<T>
        where <T as TryFrom<u64>>::Error: Debug {
    fn default() -> Atomic64Bit<T> {
        Atomic64Bit::new(T::default())
    }
}

impl<T: ?Sized+TryFrom<u64>+Into<u64>+Debug> From<T> for Atomic64Bit<T>
        where <T as TryFrom<u64>>::Error: Debug {
    fn from(v: T) -> Atomic64Bit<T> {
        Atomic64Bit::new(v)
    }
}
