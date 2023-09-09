/* Copyright (c) 2017-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines blocks of physical memory. These are basically lower-level versions of
//! `Box<[T]>`. The only abstraction they provide is automatic conversion between physical and
//! virtual addresses. Since they work with physical memory, there is no guarantee that the
//! implementation stores the contents of a block contiguously in virtual memory, or even that the
//! entire block is present in virtual memory at all times.

use {
    core::mem::{self, MaybeUninit},

    crate::phys::{
        Allocation,
        ptr::PhysPtr
    }
};

/// Represents a block of mutable physical memory.
#[derive(Debug)]
#[must_use]
pub struct BlockMut<T> {
    /// A pointer to the lower bound of the block
    base: PhysPtr<T, *mut T>,

    /// The number of `T` elements in the block
    size: usize,

    /// An object that will free this block when dropped, or `None` for a zero-sized block.
    allocation: Option<Allocation<'static>>,
}

/// A block of physical memory that is specifically for memory-mapped I/O rather than RAM.
///
/// Unlike `BlockMut`, this type is `Sync` because I/O doesn't always require synchronization
/// between CPUs. Consult the device's specification to determine its synchronization requirements.
#[derive(Debug)]
#[must_use]
pub struct Mmio<T> {
    /// A pointer to the lower bound of the block
    base: PhysPtr<T, *mut T>,

    /// The number of `T` elements in the block
    size: usize,

    /// An object that will free this block when dropped, or `None` for a zero-sized block.
    allocation: Option<Allocation<'static>>,
}

// impl<T> !Sync for BlockMut<T> {} // This type provides internal mutability with no thread-safety.
unsafe impl<T> Send for BlockMut<T> {}

unsafe impl<T> Sync for Mmio<T> {}
unsafe impl<T> Send for Mmio<T> {}

macro_rules! impl_phys_block_common {
    ( $generic:tt ) => {
        impl<T> $generic<T> {
            /// Makes a new instance of `$generic` with the given base address and size,
            /// measured in chunks of size `align_of::<T>()`.
            pub(crate) const fn new(
                    base: PhysPtr<T, *mut T>,
                    size: usize,
                    allocation: Option<Allocation<'static>>
            ) -> $generic<T> {
                $generic { base, size, allocation }
            }

            /// Returns the base address of the block.
            pub const fn base(&self) -> PhysPtr<T, *mut T> {
                self.base
            }

            /// Returns the number of `T`-sized elements within the block.
            pub const fn size(&self) -> usize {
                self.size
            }

            /// Returns a virtual reference to the given index within the block. This is just
            /// like the indexing portion of an array access: the index is given in units
            /// of the array-element size of `T`, rather than units of 1 byte. This is not an
            /// implementation of `Index` because it returns a raw pointer instead of a reference.
            ///
            /// # Panics
            /// This function panics if the given `index` is outside the bounds of the block.
            pub fn index(&self, index: usize) -> *mut T {
                self.get_ptr_phys(index).as_virt_unchecked()
            }

            /// Returns a raw physical pointer to the given index within the block. This is just
            /// like the indexing portion of an array access: the index is given in units
            /// of the array-element size of `T`, rather than units of 1 byte.
            pub fn get_ptr_phys(&self, index: usize) -> PhysPtr<T, *mut T> {
                assert!(index < self.size(), "physical memory block index out of bounds: {} {{ base: {}, size: {} }}, index = {}",
                    stringify!($generic), self.base().as_addr_phys(), self.size(), index);
                unsafe { self.base.add(index) }
            }

            /// Unsafely transmutes a `$generic<T>` into a `$generic<U>`. The contents of memory are
            /// left unchanged by this operation, so it can't guarantee in general that the `U`
            /// values will be sensible, or even valid. Always make sure you know why you need to
            /// use this function before you decide to use it.
            pub unsafe fn transmute<U>(mut self) -> $generic<U> {
                $generic {
                    base: PhysPtr::<U, *mut U>::from_addr_phys(self.base.as_addr_phys()),
                    size: self.size * mem::size_of::<T>() / mem::size_of::<U>(),
                    allocation: mem::replace(&mut self.allocation, None),
                }
            }
        }

        impl<T> Drop for $generic<T> {
            fn drop(&mut self) {
                for i in 0 .. self.size() {
                    unsafe {
                        self.index(i).drop_in_place();
                    }
                }
            }
        }
    };
}

impl_phys_block_common!(BlockMut);
impl_phys_block_common!(Mmio);

impl<T> BlockMut<MaybeUninit<T>> {
    /// Assumes everything in the block is initialized in a similar manner to
    /// `MaybeUninit::assume_init`.
    pub fn assume_init(mut self) -> BlockMut<T> {
        BlockMut {
            base: PhysPtr::<T, *mut T>::from_addr_phys(self.base.as_addr_phys()),
            size: self.size,
            allocation: mem::replace(&mut self.allocation, None),
        }
    }
}
