/* Copyright (c) 2020-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines an atomically accessible tagged pointer, suitable for things like storing
//! the pointer to the next node in a list together with a generation number to keep the list's
//! state consistent when both adding and removing elements.

#![no_std]

#![deny(warnings, missing_docs)]

#![feature(const_panic)]

use {
    core::{
        marker::PhantomData,
        mem,
        sync::atomic::{AtomicUsize, AtomicPtr, Ordering}
    }
};

/// A pointer paired with a pointer-sized tag. The pointer and tag are only ever updated at the same
/// time, using an atomic compare-and-swap operation.
#[derive(Debug)]
pub struct TaggedPtr<T> {
    // The internal representation of the tagged pointer. The physical pointer is divided by the
    // alignment of type `T` and the tag placed in the highest bits so that, when adding to the tag,
    // any overflow has no effect on the pointer.
    internal: AtomicUsize,

    _phantom: PhantomData<AtomicPtr<T>>
}

impl<T> TaggedPtr<T> {
    /// The smallest amount that can be added to the tag (based on being stored above the pointer).
    pub const TAG_UNIT: usize = usize::max_value() / mem::align_of::<T>() + 1;

    /// This function is equivalent to `new(ptr::null_mut(), tag)` except that it's `const`.
    pub const fn new_null(tag: usize) -> Self {
        // If `T` doesn't need to be aligned at all, there are no extra bits for the tag.
        assert!(mem::align_of::<T>() > 1);

        assert!(tag % Self::TAG_UNIT == 0);

        Self {
            internal: AtomicUsize::new(tag),
            _phantom: PhantomData
        }
    }

    /// Creates a new `TaggedPtr` with the given pointer and tag. The pointer must be aligned, and
    /// the tag must be a multiple of `TaggedPtr<T>::TAG_UNIT`.
    pub fn new(ptr: *mut T, tag: usize) -> Self {
        // If `T` doesn't need to be aligned at all, there are no extra bits for the tag.
        assert!(mem::align_of::<T>() > 1);

        assert_eq!(ptr as usize % mem::align_of::<T>(), 0);
        assert_eq!(tag % Self::TAG_UNIT, 0);

        Self {
            internal: AtomicUsize::new(ptr as usize / mem::align_of::<T>() + tag),
            _phantom: PhantomData
        }
    }

    /// Loads the current value of the tagged pointer.
    pub fn load(&self, ordering: Ordering) -> (*mut T, usize) {
        let raw = self.internal.load(ordering);

        let ptr = raw.wrapping_mul(mem::align_of::<T>()) as *mut T;
        let tag = raw / Self::TAG_UNIT * Self::TAG_UNIT;

        (ptr, tag)
    }

    /// Stores a value into the tagged pointer.
    pub fn store(&self, (ptr, tag): (*mut T, usize), ordering: Ordering) {
        assert_eq!(ptr as usize % mem::align_of::<T>(), 0);
        assert_eq!(tag % Self::TAG_UNIT, 0);

        let raw = ptr as usize / mem::align_of::<T>() + tag;

        self.internal.store(raw, ordering)
    }

    /// Loads the pointer and tag and adds the given `step` to the tag, in one atomic operation.
    /// The step must be a multiple of `Self::TAG_UNIT`.
    pub fn fetch_add_tag(&self, step: usize, ordering: Ordering) -> (*mut T, usize) {
        assert_eq!(step % Self::TAG_UNIT, 0);

        let raw = self.internal.fetch_add(step, ordering);

        let ptr = raw.wrapping_mul(mem::align_of::<T>()) as *mut T;
        let tag = raw / Self::TAG_UNIT * Self::TAG_UNIT;

        (ptr, tag)
    }

    /// Performs an atomic CAS operation on the tagged pointer.
    pub fn compare_exchange(
            &self,
            (old_ptr, old_tag): (*mut T, usize),
            (new_ptr, new_tag): (*mut T, usize),
            success: Ordering,
            failure: Ordering
    ) -> Result<(*mut T, usize), (*mut T, usize)> {
        assert_eq!(old_ptr as usize % mem::align_of::<T>(), 0);
        assert_eq!(new_ptr as usize % mem::align_of::<T>(), 0);
        assert_eq!(old_tag % Self::TAG_UNIT, 0);
        assert_eq!(new_tag % Self::TAG_UNIT, 0);

        let old_raw = old_ptr as usize / mem::align_of::<T>() + old_tag;
        let new_raw = new_ptr as usize / mem::align_of::<T>() + new_tag;

        match self.internal.compare_exchange(old_raw, new_raw, success, failure) {
            Ok(found_raw) => {
                let found_ptr = found_raw.wrapping_mul(mem::align_of::<T>()) as *mut T;
                let found_tag = found_raw / (usize::max_value() / mem::align_of::<T>() + 1);
                Ok((found_ptr, found_tag))
            },
            Err(found_raw) => {
                let found_ptr = found_raw.wrapping_mul(mem::align_of::<T>()) as *mut T;
                let found_tag = found_raw / (usize::max_value() / mem::align_of::<T>() + 1);
                Err((found_ptr, found_tag))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add some tests to make sure tagged pointers remain consistent between reads and writes.
    
    #[test]
    fn need_tests() {}
}
