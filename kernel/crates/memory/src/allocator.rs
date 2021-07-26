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

//! This module defines the allocator type that Rust will use in the rest of the kernel. Using it
//! as the global allocator allows most of the kernel to use abstractions like `Box`, `Vec`, and
//! `String` by importing them from the `alloc` crate.

// PERF: Implement multiple levels of heaps. The thread-safe, lock-free, linked-list-based heap we
//       have now is extremely flexible, but it seems like it would be slow for lots of small
//       allocations. Instead, we can use it to allocate large chunks of CPU-local memory that can
//       be managed by a faster heap (perhaps based on a bitmap or a stack of free pages) that
//       doesn't have to be thread-safe.

// TODO: Write a low-priority daemon that tests all of the system's RAM, and prefer RAM that's been
//       tested when fulfilling malloc requests. Don't block until the RAM is tested, though--it's
//       probably fine, and the user would notice that slow-down. Maybe we can also have a boot
//       option to test all of the system's RAM before allowing the heap to be used.

use {
    core::{
        alloc::{Layout, Allocator, AllocError},
        any::type_name,
        mem,
        num::NonZeroUsize,
        ptr::{self, NonNull}
    },

    crate::phys::{
        block::{Block, BlockMut, Mmio},
        heap,
        ptr::PhysPtr
    }
};

/// An interface to the heap. This interface allows allocating and freeing memory at the lowest
/// level. Abstractions like `Box`, `Vec`, and `String` should be preferred in general.
#[derive(Debug, Clone, Copy)]
pub struct AllMemAlloc;

impl AllMemAlloc {
    /// Immutably reserves the given number of bytes (`size`) of physical memory, starting at physical memory
    /// address `base`.
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is a `Block` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn reserve<T>(&self, base: usize, size: usize) -> Result<Block<T>, AllocError> {
        assert_eq!(
            base % mem::align_of::<T>(), 0,
            "base = {:#x}, align of {} = {:#x}", base, type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        let node = if let Some(size) = NonZeroUsize::new(size) {
            Some(heap::reserve(PhysPtr::<u8, *const _>::from_addr_phys(base), size)?)
        } else {
            None
        };
        Ok(Block::<T>::new(PhysPtr::<_, *const _>::from_addr_phys(base), size / mem::size_of::<T>(), node))
    }

    /// Mutably reserves the given number of bytes (`size`) of physical memory, starting at physical memory
    /// address `base`.
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is a `BlockMut` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn reserve_mut<T>(&self, base: usize, size: usize) -> Result<BlockMut<T>, AllocError> {
        assert_eq!(
            base % mem::align_of::<T>(), 0,
            "base = {:#x}, align of {} = {:#x}", base, type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        let node = if let Some(size) = NonZeroUsize::new(size) {
            Some(heap::reserve_mut(PhysPtr::<u8, *mut _>::from_addr_phys(base), size)?)
        } else {
            None
        };
        Ok(BlockMut::<T>::new(PhysPtr::<_, *mut _>::from_addr_phys(base), size / mem::size_of::<T>(), node))
    }

    /// Mutably reserves the given number of bytes (`size`) of memory-mapped I/O space, starting at
    /// physical memory address `base`.
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is an `Mmio` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn mmio_mut<T>(&self, base: usize, size: usize) -> Result<Mmio<T>, AllocError> {
        assert_eq!(
            base % mem::align_of::<T>(), 0,
            "base = {:#x}, align of {} = {:#x}", base, type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        let node = if let Some(size) = NonZeroUsize::new(size) {
            Some(heap::reserve_mmio(PhysPtr::<u8, *mut _>::from_addr_phys(base), size)?)
        } else {
            None
        };
        Ok(Mmio::<T>::new(PhysPtr::<_, *mut _>::from_addr_phys(base), size / mem::size_of::<T>(), node))
    }

    /// Finds and mutably reserves the given number of bytes (`size`) of physical memory, aligned on an
    /// `align`-byte boundary.
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is a `BlockMut` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn malloc<T>(&self, size: usize, align: NonZeroUsize) -> Result<BlockMut<T>, AllocError> {
        assert_eq!(
            align.get() % mem::align_of::<T>(), 0,
            "align = {:#x}, align of {} = {:#x}", align.get(), type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        let (base, node) = if let Some(size) = NonZeroUsize::new(size) {
            let (ptr, node) = heap::malloc(size, align)?;
            (ptr.as_addr_phys(), Some(node))
        } else {
            // No need to touch the heap if we're allocating zero bytes.
            (align.get(), None)
        };
        Ok(BlockMut::<T>::new(PhysPtr::<_, *mut _>::from_addr_phys(base), size / mem::size_of::<T>(), node))
    }

    /// Finds and mutably reserves the given number of bytes (`size`) of physical memory, aligned on an
    /// `align`-byte boundary and only using up to `max_bits` bits for the address. (For instance, if
    /// `max_bits` is equal to 20, only the first 1 MiB of physical memory will be considered for
    /// allocation.)
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is a `BlockMut` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn malloc_low<T>(&self, size: usize, align: NonZeroUsize, max_bits: usize) -> Result<BlockMut<T>, AllocError> {
        assert_eq!(
            align.get() % mem::align_of::<T>(), 0,
            "align = {:#x}, align of {} = {:#x}", align.get(), type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        if max_bits < mem::size_of::<usize>() * 8 && align.get() > (1 << max_bits) {
            // There's nothing we could return with this alignment besides null!
            return Err(AllocError);
        }

        let (base, node) = if let Some(size) = NonZeroUsize::new(size) {
            let (ptr, node) = heap::malloc_low(size, align, max_bits)?;
            (ptr.as_addr_phys(), Some(node))
        } else {
            // No need to touch the heap if we're allocating zero bytes.
            (align.get(), None)
        };
        Ok(BlockMut::<T>::new(PhysPtr::<_, *mut _>::from_addr_phys(base), size / mem::size_of::<T>(), node))
    }

    /// Frees the allocated block at the virtual address that the given pointer indicates. This
    /// function is almost never needed because every `Block` and `BlockMut` frees its memory when
    /// dropped. To avoid a double-free, only call this function if you've called `mem::forget` on
    /// the block.
    pub fn free<T>(&self, ptr: *mut T) {
        // TODO: It might be useful to keep a hash table of recently allocated blocks. Then, we
        // could find those ones in constant time and only use this linear-time alternative as a
        // fallback for when the relevant entry in that table has been overwritten. The hash table
        // should be stored in the AllMemAlloc, not the heap, because that optimization will be useful
        // only for blocks that have been allocated by `Allocator::allocate`. In all other cases, we can
        // assume the block will be dropped, which will deallocate it.
        heap::dealloc(PhysPtr::<_, *mut _>::from_virt(ptr as *mut u8))
    }
}

/// This trait is required for Rust to treat this type as an allocator. It should not be used
/// directly. Instead, use the functions provided by the type itself rather than this trait. They
/// provide more functionality, are more efficient, and don't require `unsafe` blocks.
unsafe impl Allocator for AllMemAlloc {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if let Ok(block) = self.malloc(layout.size(),
                NonZeroUsize::new(layout.align()).map_or(Err(AllocError), |x| Ok(x))?) {
            let ptr = block.base().as_virt_unchecked();
            let size = block.size();
            mem::forget(block);
            Ok(unsafe { NonNull::slice_from_raw_parts(NonNull::new_unchecked(ptr), size) })
        } else {
            // TODO: Do everything you can to free some memory instead of returning Err. Anything
            // is better than having the kernel panic, even if it means crashing a program. Maybe
            // just freeze this thread until the memory is available, but that could lead to
            // deadlock. Maybe freeze this thread unless it's the only thread currently running, in
            // which case we should crash the thread and wake another up.
            //
            // Better idea:
            // loop {
            //     loop {
            //         try to swap some of this thread's memory to the disk;
            //         if successful {
            //             try allocating again;
            //             if successful {
            //                 return Ok;
            //             }
            //         } else {
            //             break;
            //         }
            //     }
            //     if number of unfrozen threads > 1 {
            //         // We failed to swap out some memory, so let's try freezing the thread.
            //         freeze this thread;
            //         try allocating again;
            //         if successful {
            //             return Ok;
            //         }
            //     } else {
            //         // Every thread is waiting to allocate some memory. Sacrifice this thread so the
            //         // kernel doesn't have to panic as long as at least one thread exists.
            //         kill thread;
            //     }
            // }

            Err(AllocError)
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        self.free(ptr.as_ptr())
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout)
            -> Result<NonNull<[u8]>, AllocError> {
        // The heap implementation doesn't allow us to make a block larger, so allocate a whole new
        // block instead.
        let new_ptr = self.allocate(new_layout)?;
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), old_layout.size());
        self.deallocate(ptr, old_layout);

        Ok(new_ptr)
    }

    unsafe fn grow_zeroed(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout)
            -> Result<NonNull<[u8]>, AllocError> {
        let mut new_slice = self.grow(ptr, old_layout, new_layout)?;
        new_slice.as_mut()[old_layout.size() .. ].fill(0);

        Ok(new_slice)
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout)
            -> Result<NonNull<[u8]>, AllocError> {
        let phys_ptr = PhysPtr::<_, *const _>::from_virt(ptr.as_ptr());
        if phys_ptr.as_addr_phys() % new_layout.align() == 0 {
            // It's UB for new_size to be greater than the block's current size, so this is safe.
            Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()))
        } else {
            // A new, incompatible alignment is required, so allocate a new block.
            let new_ptr = self.allocate(new_layout)?;
            ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), new_layout.size());
            self.deallocate(ptr, old_layout);
            
            Ok(new_ptr)
        }
    }
}
