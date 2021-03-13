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

//! This module defines blocks of physical memory. These are basically lower-level versions of
//! `Box<[T]>`. The only abstraction they provide is automatic conversion between physical and
//! virtual addresses. Since they work with physical memory, there is no guarantee that the
//! implementation stores the contents of a block contiguously in virtual memory, or even that the
//! entire block is present in virtual memory at all times.

use {
    core::mem,

    i18n::Text,

    crate::phys::{
        heap::Allocation,
        ptr::PhysPtr
    }
};

/// Represents a block of physical memory.
///
/// This struct is for an immutable block of memory. For the mutable version, see `BlockMut`.
#[derive(Debug)]
#[must_use]
pub struct Block<T> {
    /// A pointer to the lower bound of the block
    base: PhysPtr<T, *const T>,

    /// The number of `T` elements in the block
    size: usize,

    /// An object that will free this block when dropped, or `None` for a zero-sized block.
    allocation: Option<Allocation>
}

/// The same as `Block`, except that the contents of the block are mutable.
#[derive(Debug)]
#[must_use]
pub struct BlockMut<T> {
    /// A pointer to the lower bound of the block
    base: PhysPtr<T, *mut T>,

    /// The number of `T` elements in the block
    size: usize,

    /// An object that will free this block when dropped, or `None` for a zero-sized block.
    allocation: Option<Allocation>
}

/// A block of physical memory that is specifically for memory-mapped I/O rather than actual
/// memory.
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
    allocation: Option<Allocation>
}

unsafe impl<T> Sync for Block<T> {}
unsafe impl<T> Send for Block<T> {}

// impl<T> !Sync for BlockMut<T> {} // This type provides internal mutability with no thread-safety.
unsafe impl<T> Send for BlockMut<T> {}

unsafe impl<T> Sync for Mmio<T> {}
unsafe impl<T> Send for Mmio<T> {}

macro_rules! impl_phys_block_common {
    ( $generic:tt, $ptr_mutability:tt ) => {
        impl<T> $generic<T> {
            /// Makes a new instance of `$generic` with the given base address and size,
            /// measured in chunks of size `align_of::<T>()`.
            pub(crate) const fn new(base: PhysPtr<T, *$ptr_mutability T>, size: usize, allocation: Option<Allocation>) -> $generic<T> {
                $generic {
                    base,
                    size,
                    allocation
                }
            }

            /// Returns the base address of the block.
            pub const fn base(&self) -> PhysPtr<T, *$ptr_mutability T> {
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
            pub fn index(&self, index: usize) -> *$ptr_mutability T {
                self.get_ptr_phys(index).as_virt_unchecked()
            }

            /// Returns a raw physical pointer to the given index within the block. This is just
            /// like the indexing portion of an array access: the index is given in units
            /// of the array-element size of `T`, rather than units of 1 byte.
            pub fn get_ptr_phys(&self, index: usize) -> PhysPtr<T, *$ptr_mutability T> {
                assert!(index < self.size(),
                    "{}", Text::PhysBlockIndexOOB(self.base.as_addr_phys(), self.size(), index));
                unsafe { self.base.add(index) }
            }
        }
    };
}

impl_phys_block_common!(Block, const);
impl_phys_block_common!(BlockMut, mut);
impl_phys_block_common!(Mmio, mut);

impl<T> From<BlockMut<T>> for Block<T> {
    fn from(mut block: BlockMut<T>) -> Block<T> {
        let allocation = mem::replace(&mut block.allocation, None);
        Block::new(PhysPtr::<_, *const _>::from_addr_phys(block.base.as_addr_phys()), block.size, allocation)
    }
}
