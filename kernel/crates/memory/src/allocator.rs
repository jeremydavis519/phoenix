/* Copyright (c) 2019-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

// TODO: Write a low-priority daemon that tests all of the system's RAM, and prefer RAM that's been
//       tested when fulfilling malloc requests. Don't block until the RAM is tested, though--it's
//       probably fine, and the user would notice that slow-down. Maybe we can also have a boot
//       option to test all of the system's RAM before allowing the heap to be used.

use {
    alloc::alloc::{Layout, Allocator, AllocError},
    core::{
        any::type_name,
        mem::{self, MaybeUninit},
        num::NonZeroUsize,
        ptr::{self, NonNull},
        sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    },

    libphoenix::{profiler_probe, profiler_setup},

    crate::{
        phys::{
            Allocation,
            block::{BlockMut, Mmio},
            heap,
            ptr::PhysPtr,
            slab::SlabAllocator,
        },
        virt::paging::page_size,
    },
};

profiler_setup!();

/// An interface to the heap. This interface allows allocating and freeing memory at the lowest
/// level. Abstractions like `Box`, `Vec`, and `String` should be preferred in general.
#[derive(Debug, Clone, Copy)]
pub struct AllMemAlloc;

impl AllMemAlloc {
    /// Mutably reserves the given number of bytes (`size`) of memory-mapped I/O space, starting at
    /// physical memory address `base`.
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is an `Mmio` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn mmio_mut<T>(&self, base: usize, size: usize) -> Result<Mmio<T>, AllocError> {
        profiler_probe!(=> ENTRANCE);
        assert_eq!(
            base % mem::align_of::<T>(), 0,
            "base = {:#x}, align of {} = {:#x}", base, type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        let allocation = if let Some(size) = NonZeroUsize::new(size) {
            Some(Allocation::Heap(heap::reserve_mmio(PhysPtr::<u8, *mut _>::from_addr_phys(base), size)?))
        } else {
            None
        };
        let block = Mmio::<T>::new(
            PhysPtr::<_, *mut _>::from_addr_phys(base),
            size / mem::size_of::<T>(),
            allocation,
        );
        profiler_probe!(ENTRANCE);
        Ok(block)
    }

    /// Finds and mutably reserves the given number of bytes (`size`) of physical memory, aligned on an
    /// `align`-byte boundary.
    ///
    /// # Returns
    ///   * `Ok(block)` if the block was successfully reserved, where `block` is a `BlockMut` representing
    ///         the reserved block.
    ///   * `Err(AllocErr)` on failure.
    pub fn malloc<T>(&self, size: usize, align: NonZeroUsize) -> Result<BlockMut<MaybeUninit<T>>, AllocError> {
        profiler_probe!(=> ENTRANCE);
        assert_eq!(
            align.get() % mem::align_of::<T>(), 0,
            "align = {:#x}, align of {} = {:#x}", align.get(), type_name::<T>(), mem::align_of::<T>()
        );
        assert!(
            size >= mem::size_of::<T>(),
            "size = {:#x}, size of {} = {:#x}", size, type_name::<T>(), mem::size_of::<T>()
        );

        let page_size = page_size();
        if size == page_size && align.get() == page_size {
            // Try using a slab allocator anytime we need a page.
            let mut allocators = SLAB_ALLOCATORS.load(Ordering::Acquire);
            while !allocators.is_null() {
                let allocator = unsafe { &(*allocators).head };
                match allocator.try_alloc() {
                    Ok(block) => {
                        profiler_probe!(ENTRANCE);
                        return unsafe { Ok(BlockMut::transmute(block)) };
                    },
                    Err(_) => allocators = unsafe { (*allocators).tail.load(Ordering::Acquire) },
                };
            }

            // Try making a new slab allocator if none of the existing ones could take our request.
            let mut allocators = SLAB_ALLOCATORS.load(Ordering::Acquire);
            if let Ok(new_allocator_node) = SlabAllocatorList::try_new(allocators) {
                let new_allocator_node_ptr = new_allocator_node as *mut _;

                // Allocating a slab can't fail now because the only reference to the allocator is
                // stored in a local variable and we know that it has some free slabs.
                let block = new_allocator_node.head.try_alloc()
                    .expect("new slab allocator failed to allocate a slab");

                // Prepend the new allocator to the list. (This is better than appending because it
                // ensures that the first allocator in the list will usually have some free slabs.)
                loop {
                    match SLAB_ALLOCATORS.compare_exchange_weak(
                            allocators,
                            new_allocator_node_ptr,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                    ) {
                        Ok(_) => break,
                        Err(x) => {
                            allocators = x;
                            new_allocator_node.tail.store(allocators, Ordering::Release);
                        },
                    };
                }

                profiler_probe!(ENTRANCE);
                return unsafe { Ok(BlockMut::transmute(block)) };
            }
        }

        // As a last resort, try allocating directly from the heap.
        let (base, allocation) = if let Some(size) = NonZeroUsize::new(size) {
            let (ptr, allocation) = heap::malloc(size, align)?;
            (ptr.as_addr_phys(), Some(Allocation::Heap(allocation)))
        } else {
            // No need to touch the heap if we're allocating zero bytes.
            (align.get(), None)
        };
        let block = BlockMut::<MaybeUninit<T>>::new(
            PhysPtr::<_, *mut _>::from_addr_phys(base),
            size / mem::size_of::<T>(),
            allocation,
        );
        profiler_probe!(ENTRANCE);
        Ok(block)
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
    pub fn malloc_low<T>(&self, size: usize, align: NonZeroUsize, max_bits: usize)
            -> Result<BlockMut<MaybeUninit<T>>, AllocError> {
        profiler_probe!(=> ENTRANCE);
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

        let (base, allocation) = if let Some(size) = NonZeroUsize::new(size) {
            let (ptr, allocation) = heap::malloc_low(size, align, max_bits)?;
            (ptr.as_addr_phys(), Some(Allocation::Heap(allocation)))
        } else {
            // No need to touch the heap if we're allocating zero bytes.
            (align.get(), None)
        };
        let block = BlockMut::<MaybeUninit<T>>::new(
            PhysPtr::<_, *mut _>::from_addr_phys(base),
            size / mem::size_of::<T>(),
            allocation,
        );
        profiler_probe!(ENTRANCE);
        Ok(block)
    }

    /// Frees the allocated block at the virtual address that the given pointer indicates. This
    /// function is almost never needed because every `Block` and `BlockMut` frees its memory when
    /// dropped. To avoid a double-free, only call this function if you've called `mem::forget` on
    /// the block.
    pub fn free<T>(&self, ptr: *mut T) {
        profiler_probe!(=> ENTRANCE);

        let phys_ptr = PhysPtr::<_, *mut _>::from_virt(ptr as *mut u8);
        let phys_addr = phys_ptr.as_addr_phys();

        // TODO: It might be useful to keep a hash table of recently allocated blocks. Then, we
        // could find those ones in constant time and only use this linear-time alternative as a
        // fallback for when the relevant entry in that table has been overwritten. The hash table
        // should be stored in the AllMemAlloc, not the heap, because that optimization will be useful
        // only for blocks that have been allocated by `Allocator::allocate`. In all other cases, we can
        // assume the block will be dropped, which will deallocate it.

        // Free the block with a slab allocator if it was allocated from one.
        let mut allocators = SLAB_ALLOCATORS.load(Ordering::Acquire);
        while !allocators.is_null() {
            unsafe {
                if (*allocators).head.owns_slab(phys_addr) {
                    (*allocators).head.free(phys_addr);
                    profiler_probe!(ENTRANCE);
                    return;
                }
                allocators = (*allocators).tail.load(Ordering::Acquire);
            }
        }

        // Otherwise, the block must have been allocated directly on the heap.
        heap::dealloc(phys_ptr);
        profiler_probe!(ENTRANCE);
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
            unsafe {
                ptr.write(MaybeUninit::uninit());
                Ok(NonNull::slice_from_raw_parts(NonNull::new_unchecked((*ptr).as_mut_ptr()), size))
            }
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
        profiler_probe!(=> ENTRANCE);

        // The heap implementation doesn't allow us to make a block larger, so allocate a whole new
        // block instead.
        let new_ptr = self.allocate(new_layout)?;
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), old_layout.size());
        self.deallocate(ptr, old_layout);

        profiler_probe!(ENTRANCE);
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
        profiler_probe!(=> ENTRANCE);
        let phys_ptr = PhysPtr::<_, *const _>::from_virt(ptr.as_ptr());
        if phys_ptr.as_addr_phys() % new_layout.align() == 0 {
            // It's UB for new_size to be greater than the block's current size, so this is safe.
            profiler_probe!(ENTRANCE);
            Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()))
        } else {
            // A new, incompatible alignment is required, so allocate a new block.
            let new_ptr = self.allocate(new_layout)?;
            ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), new_layout.size());
            self.deallocate(ptr, old_layout);

            profiler_probe!(ENTRANCE);
            Ok(new_ptr)
        }
    }
}

// Slab allocators for fast page-sized allocations

#[derive(Debug)]
struct SlabAllocatorList {
    head: SlabAllocator,
    tail: AtomicPtr<SlabAllocatorList>,

    // This must be last to ensure it is dropped last.
    _self_allocation: heap::Allocation,
}

static SLAB_ALLOCATORS: AtomicPtr<SlabAllocatorList> = AtomicPtr::new(ptr::null_mut());
static MAX_SLABS_PER_ALLOCATOR: usize = 256;

impl SlabAllocatorList {
    fn try_new(tail: *mut SlabAllocatorList) -> Result<&'static mut Self, AllocError> {
        let page_size = page_size();

        // Allocate an arena and a buffer for as many slabs as possible.
        let mut slabs_count = MAX_SLABS_PER_ALLOCATOR;
        loop {
            match heap::malloc(
                        unsafe { NonZeroUsize::new_unchecked(slabs_count * page_size) },
                        unsafe { NonZeroUsize::new_unchecked(page_size) },
                    )
                    .map(|(arena_ptr, allocation)| BlockMut::<u8>::new(
                        PhysPtr::<_, *mut _>::from_addr_phys(arena_ptr.as_addr_phys()),
                        slabs_count * page_size,
                        Some(Allocation::Heap(allocation)),
                    ))
                    .and_then(|arena|
                        heap::malloc(
                            unsafe { NonZeroUsize::new_unchecked(mem::size_of::<AtomicUsize>() * slabs_count) },
                            unsafe { NonZeroUsize::new_unchecked(mem::align_of::<AtomicUsize>()) },
                        )
                        .map(|(buf_ptr, allocation)| BlockMut::<AtomicUsize>::new(
                            PhysPtr::<_, *mut _>::from_addr_phys(buf_ptr.as_addr_phys()),
                            slabs_count,
                            Some(Allocation::Heap(allocation)),
                        ))
                        .map(|buffer| (arena, buffer))
                    )
                    .and_then(|(arena, buffer)|
                        heap::malloc(
                            unsafe { NonZeroUsize::new_unchecked(mem::size_of::<Self>()) },
                            unsafe { NonZeroUsize::new_unchecked(mem::align_of::<Self>()) },
                        )
                        .map(|(allocs_ptr, allocation)| (arena, buffer, allocs_ptr, allocation))
                    ) {
                Ok((arena, buffer, allocs_ptr, allocation)) => {
                    let allocators = allocs_ptr.as_virt_unchecked().cast::<SlabAllocatorList>();
                    unsafe {
                        allocators.write(
                            Self {
                                head: SlabAllocator::new(arena, buffer, NonZeroUsize::new_unchecked(page_size)),
                                tail: AtomicPtr::new(tail),
                                _self_allocation: allocation,
                            },
                        );
                    }
                    return Ok(unsafe { &mut *allocators });
                },
                Err(AllocError) => {
                    if slabs_count == 1 {
                        // We can't even make a slab allocator for one page.
                        return Err(AllocError);
                    }
                    // Retry with a smaller number of slabs.
                    slabs_count /= 2;
                    continue;
                },
            };
        }
    }
}

impl Drop for SlabAllocatorList {
    fn drop(&mut self) {
        let tail = self.tail.load(Ordering::Acquire);
        if !tail.is_null() {
            unsafe { tail.drop_in_place(); }
        }
    }
}

// TODO: Bump allocators for smaller-than-page-sized allocations
//       Make a bump allocator from a slab for smaller-than-a-page allocations. Keep using that bump
//       allocator until the next allocation would overflow, then make a new one. After a given bump
//       allocator has been made obsolete in this way, whenever everything in that slab has been freed,
//       free the slab. A bump allocator can handle deallocation just by adding to an atomic number of
//       bytes freed.
