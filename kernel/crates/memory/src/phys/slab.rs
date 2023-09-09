/* Copyright (c) 2022-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the kernel's slab allocators, which are intended to increase the speed of
//! memory allocation by working in constant time. The optimization should work especially well for
//! allocating new pages for userspace, since it can always be done exactly one page at a time.

use {
    core::{
        num::NonZeroUsize,
        sync::atomic::{AtomicUsize, Ordering},
    },

    locks::Mutex,

    crate::phys::{
        block::BlockMut,
        ptr::PhysPtr,
    },
};

/// A slab allocator. Allocation and deallocation are guaranteed to work in constant time, although
/// allocation is not guaranteed to succeed.
#[derive(Debug)]
pub struct SlabAllocator {
    arena:               BlockMut<u8>,
    slabs_buf:           BlockMut<AtomicUsize>,
    slab_size:           NonZeroUsize,
    first_free_slab_idx: Mutex<usize>,
    first_used_slab_idx: AtomicUsize,
}

unsafe impl Sync for SlabAllocator {}

const USED_SLAB: usize = usize::max_value();

impl SlabAllocator {
    /// Makes a new slab allocator by dividing the given arena into slabs of the given size.
    ///
    /// Panics if
    /// * the `arena.size()` is not equal to `slab_size * slab_buf.size()`, or
    /// * `slabs_buf.size()` is not a power of 2.
    ///
    /// There is no alignment requirement, but the slabs will have exactly the same alignment as the
    /// arena. As such, the caller is responsible for aligning the arena to the same standard as the
    /// slabs will need. For instance, if a slab allocator is used for allocating pages, the whole
    /// arena must be page-aligned.
    pub fn new(
            arena:     BlockMut<u8>,
            slabs_buf: BlockMut<AtomicUsize>,
            slab_size: NonZeroUsize,
    ) -> Self {
        assert_eq!(
            arena.size(),
            slab_size.get() * slabs_buf.size(),
            "arena of {} bytes can't be made of {} slabs of {} bytes each",
            arena.size(), slabs_buf.size(), slab_size,
        );
        let slabs_count = slabs_buf.size();
        assert_eq!(slabs_count.count_ones(), 1, "max slab count of {slabs_count} is not a power of 2");

        for i in 0 .. slabs_count {
            unsafe {
                (*slabs_buf.index(i)).store(arena.base().as_addr_phys() + i * slab_size.get(), Ordering::Release);
            }
        }

        Self {
            arena,
            slabs_buf,
            slab_size,
            first_free_slab_idx: Mutex::new(0),
            first_used_slab_idx: AtomicUsize::new(0),
        }
    }

    /// Tries to allocate a slab without blocking. Returns the address of the slab. If this fails with
    /// `SlabAllocError::Empty`, the caller should try a different allocator. If it fails with
    /// `SlabAllocError::Locked`, the caller can try again with this allocator or with a different one.
    pub fn try_alloc(&'static self) -> Result<BlockMut<u8>, SlabAllocError> {
        let mut idx = self.first_free_slab_idx.try_lock()
            .map_err(|()| SlabAllocError::Locked)?;

        let base = unsafe { (*self.slabs_buf.index(*idx)).swap(USED_SLAB, Ordering::AcqRel) };
        if base == USED_SLAB {
            return Err(SlabAllocError::Empty);
        }
        *idx = (*idx + 1) % self.slabs_buf.size();

        Ok(BlockMut::new(
            PhysPtr::<_, *mut _>::from_addr_phys(base),
            self.slab_size.get(),
            Some(super::Allocation::Slab(Allocation { allocator: self, base })),
        ))
    }

    /// Frees a slab that was previously allocated.
    ///
    /// Panics if
    /// * the given base address is not the base address of one of this allocator's slabs, or
    /// * the allocator overflows with free slabs (which can only happen as a result of a double-
    ///   free error).
    pub fn free(&self, base: usize) {
        assert!(self.owns_slab(base), "attempted to free an unowned slab");
        let idx = self.first_used_slab_idx.fetch_add(1, Ordering::AcqRel) % self.slabs_buf.size();
        let old_slab = unsafe { (*self.slabs_buf.index(idx)).swap(base, Ordering::AcqRel) };
        assert_eq!(old_slab, USED_SLAB, "slab allocator overflowed")
    }

    /// Determines whether this slab allocator owns a slab with the given base address.
    pub fn owns_slab(&self, base: usize) -> bool {
        let arena_base = self.arena.base().as_addr_phys();
        base >= arena_base && base < arena_base + self.arena.size()
            && (base - arena_base) % self.slab_size.get() == 0
    }
}

/// An RAII guard that frees an allocated slab when it is dropped.
#[derive(Debug)]
pub struct Allocation<'a> {
    allocator: &'a SlabAllocator,
    base:      usize,
}

impl<'a> Drop for Allocation<'a> {
    fn drop(&mut self) {
        self.allocator.free(self.base);
    }
}

/// An error that can be returned when trying to allocate a slab.
#[derive(Debug, Clone, Copy)]
pub enum SlabAllocError {
    /// The allocator is all out of slabs.
    Empty,
    /// Another thread holds this allocator's lock.
    Locked,
}
