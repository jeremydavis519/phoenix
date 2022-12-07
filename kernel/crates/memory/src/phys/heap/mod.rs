/* Copyright (c) 2018-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the kernel's heap and the functions needed to allocate and deallocate blocks
//! within it. From the kernel's perspective, _everything_ that is not statically allocated as part
//! of the kernel's binary is part of the heap. This definition may be different than in other
//! kernels. For instance, when a program is loaded into memory to be executed, it is placed into
//! one or more blocks in the heap.
//!
//! All addresses used in this module are physical addresses. The rest of the kernel should use the
//! global allocator (defined in the root of this crate) to interface with it. In fact, nothing in
//! this module is available to other crates.
//!
//! Care has been taken to ensure that the algorithms here work correctly in a lock-free manner.
//! It's still not real-time--some operations have unbounded time complexity--but it is, in
//! general, impossible for one thread to obtain a lock, be preempted by the hypervisor (if there
//! is one), and cause other threads to wait for it to finish. The only exception to this guarantee
//! is that the heap can only be traversed by a certain number of visitors at once--31 at the time
//! of writing. Any further visitors will block to avoid overflowing reference counts.

pub(crate) mod node;

use {
    alloc::alloc::AllocError,
    core::{
        cell::Cell,
        mem::{self, align_of, size_of, MaybeUninit},
        num::NonZeroUsize,
        pin::Pin,
        ptr,
        sync::atomic::{AtomicUsize, Ordering}
    },

    i18n::Text,
    tagged_ptr::TaggedPtr,
    crate::phys::{
        ptr::PhysPtr,
        map::{MemoryMap, Region, MEMORY_MAP}
    },
    self::node::{
        Nodes,
        MasterBlock,
        Node,
        NodeRef,

        NODES_PER_MASTER_BLOCK
    }
};

// Correct lockless operation requires the pointer to a node and its reference count to fit in one
// `usize` value. Enforcing a maximum number of simultaneous visitors ensures that the reference count
// doesn't overflow.
const MAX_VISITORS: usize = align_of::<Node>() / 2 - 1;

static STATIC_MASTER_BLOCK:  MasterBlock     = MasterBlock::new(None);
static HEAD_PTR:             TaggedPtr<Node> = TaggedPtr::new_null(0);
static EXPECTED_MALLOC_SIZE: AtomicUsize     = AtomicUsize::new(0);
static VISITORS:             AtomicUsize     = AtomicUsize::new(0);
static UNUSED_NODE_SLOTS:    AtomicUsize     = AtomicUsize::new(NODES_PER_MASTER_BLOCK);

/// Reserves a block of memory-mapped I/O starting at `base` that is `size` bytes long.
///
/// # Returns
///   * `Ok(allocation)` if the memory was reserved, where `allocation` is an object that frees the
///     block when it is dropped
///   * `Err(AllocErr)`
pub(crate) fn reserve_mmio(
        base: PhysPtr<u8, *mut u8>,
        size: NonZeroUsize
) -> Result<Allocation, AllocError> {
    reserve_mmio_with_map(base, size, &*MEMORY_MAP)
}

fn reserve_mmio_with_map(
        base: PhysPtr<u8, *mut u8>,
        size: NonZeroUsize,
        map: &MemoryMap
) -> Result<Allocation, AllocError> {
    let base = base.as_addr_phys();

    // Don't accept a request for a block that wraps around the address space. Any such request
    // is most likely an error anyway.
    if base.checked_add(size.get()).is_none() {
        return Err(AllocError);
    }

    alloc_masters(map)?;
    allocate(base, size)
}

/// Finds and reserves a block of RAM that is `size` bytes long and aligned on an `align`-byte boundary.
///
/// # Returns
///   * `Ok((ptr, allocation))` if the memory was reserved, where `ptr` is a physical pointer to the
///     first byte in the block and `allocation` is an object that frees the block when it is dropped
///   * `Err(AllocErr)`
pub(crate) fn malloc(
        size: NonZeroUsize,
        align: NonZeroUsize
) -> Result<(PhysPtr<u8, *mut u8>, Allocation), AllocError> {
    malloc_with_map(size, align, &*MEMORY_MAP)
}

fn malloc_with_map(
        size: NonZeroUsize,
        align: NonZeroUsize,
        map: &MemoryMap
) -> Result<(PhysPtr<u8, *mut u8>, Allocation), AllocError> {
    // TODO: Prefer memory regions that are not hotpluggable.

    update_expected_malloc_size(size.get());

    // Loop to handle race conditions where multiple threads try to allocate the same memory at
    // the same time.
    loop {
        alloc_masters(map)?;
        let base = find_best_base(size.get(), align.get(), map)?;
        if let Ok(allocation) = allocate(base, size) {
            return Ok((PhysPtr::<_, *mut _>::from_addr_phys(base), allocation));
        }
    }
}

/// Finds and reserves a block of RAM that is `size` bytes long and aligned on an `align`-byte boundary.
/// Does not reserve any memory with a physical address that uses more than `max_bits` bits.
///
/// # Returns
///   * `Ok((ptr, allocation))` if the memory was reserved, where `ptr` is a physical pointer to the
///     first byte in the block and `allocation` is an object that frees the block when it is dropped
///   * `Err(AllocErr)`
pub(crate) fn malloc_low(
        size: NonZeroUsize,
        align: NonZeroUsize,
        max_bits: usize
) -> Result<(PhysPtr<u8, *mut u8>, Allocation), AllocError> {
    if max_bits >= mem::size_of::<usize>() * 8 {
        return malloc(size, align);
    }

    let max_addr = 1 << max_bits;
    let mut map = MEMORY_MAP.clone();
    let _ = map.remove_region(max_addr, NonZeroUsize::new(0_usize.wrapping_sub(max_addr)).unwrap());
    let (ptr, allocation) = malloc_with_map(size, align, &map)?;
    if ptr.as_addr_phys().saturating_add(size.get()) > max_addr {
        // This can only happen if removing the region from the map failed.
        return Err(AllocError);
    }
    Ok((ptr, allocation))
}

/// Deallocates the block of memory starting at the given base address. Dropping the allocated
/// block should be preferred, as it's faster and better at preventing memory leaks, but this
/// method is required for Rust's standard interface for allocators.
pub(crate) fn dealloc(base: PhysPtr<u8, *mut u8>) {
    let base = base.as_addr_phys();

    if let Some(node) = nodes()
            .find(|node| node.base() == base) {
        node.free();
    } else {
        panic!("{}", Text::TriedToFreeNothing(base as *mut u8));
    }
}

// Finds the base of the best free block in which to allocate the given amount of memory, if
// there's enough free space anywhere.
fn find_best_base(size: usize, align: usize, map: &MemoryMap) -> Result<usize, AllocError> {
    // TODO: This is better than it was, but it could still use some refactoring.

    let mut best_base = 0;
    let mut best_size = 0;
    let mut best_score = usize::max_value();

    let mut nodes = nodes();
    let mut prev_base = 0_usize;
    let mut prev_size = 0_usize;
    for region in map.present_regions() {
        // We'll keep retrying as long as there are more blocks in this region.
        while !prev_base.checked_add(prev_size).is_none() &&       // While the free block is not past the address space
                (region.base.checked_add(region.size).is_none() ||
                    prev_base + prev_size < region.base + region.size) {    // or after the end of the region
            // Get the base and size of the next allocated block.
            let (next_base, next_size) = match nodes.next() {
                Some(node) => {
                    let size = node.size();
                    (Some(node.base()), size)
                },
                None => (None, 0)
            };

            let (free_base, free_size) = find_free_space(&region, (prev_base, prev_size), next_base, align);

            if free_size >= size {
                // Determine whether this is the best fit so far.
                if free_size == size {
                    // Perfect fit! No need to keep looking.
                    return Ok(free_base);
                }
                let score = fit_score(size, free_size);
                if score <= best_score {
                    best_base = free_base;
                    best_size = free_size;
                    best_score = score;
                }
            }

            if let Some(next_base) = next_base {
                // Move to the next free block.
                prev_base = next_base;
                prev_size = next_size;
            } else {
                // There is no next free block. Try the next region.
                break;
            }
        }
    }

    if best_size >= size {
        Ok(best_base)
    } else {
        Err(AllocError)
    }
}

// Finds the base address and size of the space between the two given blocks in the given
// region after applying the given alignment.
fn find_free_space(region: &Region, (prev_base, prev_size): (usize, usize),
        next_base: Option<usize>, align: usize) -> (usize, usize) {
    // If the free block isn't in this region, return a zero-sized block.
    if prev_base.checked_add(prev_size).is_none() ||   // Free block begins after the end of the address space
            (!region.base.checked_add(region.size).is_none() &&
                prev_base + prev_size >= region.base + region.size) {   // or after the end of the region
        return (0, 0);
    }
    if let Some(next_base) = next_base {
        if next_base <= region.base {              // Free block ends before the region
            return (0, 0);
        }
    }

    // Find the free block in this region between these allocated blocks and see how big it is.
    let mut free_base;
    let mut free_size;
    if prev_base + prev_size > region.base {    // Free block begins inside the region
        free_base = align_up(prev_base + prev_size, align);
    } else {                                    // Free block begins at or before the region's base
        free_base = align_up(region.base, align);
    }
    if let Some(next_base) = next_base {    // Free block ends inside the address space
        if region.base.checked_add(region.size).is_none() ||
                next_base < region.base + region.size { // Free block ends inside the region
            free_size = next_base.saturating_sub(free_base);
        } else {                                        // Free block ends at or before the region's end
            free_size = (region.base + region.size).saturating_sub(free_base);
        }
    } else {    // Free block ends at the top of the address space
        if region.base.checked_add(region.size).is_none() {    // Region ends at the top of the address space
            free_size = 0_usize.wrapping_sub(free_base);

            // This case would break very slightly if we handled null in the same way as we do
            // in every other case.
            if free_base == 0 {
                free_base += align;
                free_size = free_size.wrapping_sub(align);
            }
        } else {                                            // Region ends inside the address space
            free_size = (region.base + region.size).saturating_sub(free_base);
        }
    }

    // `malloc` should never allocate a block that contains null.
    if free_base == 0 {
        free_base += align;
        free_size = free_size.saturating_sub(align);
    }

    (free_base, free_size)
}

// Updates the expected size to be given to future calls to malloc with a new datapoint.
fn update_expected_malloc_size(new_size: usize) {
    // TODO: Keep track of more than one.
    EXPECTED_MALLOC_SIZE.store(new_size, Ordering::Release);
}

// Calculates a score to determine how well a given allocation would fit in a given free block.
// A lower score indicates a better fit.
fn fit_score(size: usize, free_size: usize) -> usize {
    let expected_malloc_size = EXPECTED_MALLOC_SIZE.load(Ordering::Acquire);
    let score = (free_size - size) % expected_malloc_size;
    usize::min(score, expected_malloc_size - score)
}

// Attempts to allocate a block of `size` bytes starting at `base`. Fails if the block overlaps
// at least one byte that has already been allocated. This should only be called right after
// `alloc_masters`, to make sure there are enough node slots available.
fn allocate(base: usize, size: NonZeroUsize) -> Result<Allocation, AllocError> {
    UNUSED_NODE_SLOTS.fetch_sub(1, Ordering::AcqRel);
    let (cell, master) = claim_slot();
    cell.set(MaybeUninit::new(Node::new(base, size, master)));

    // Add the node to the list.
    let node = unsafe { Pin::new_unchecked((*cell.as_ptr()).assume_init_ref()) };
    if let Err(AllocError) = node.add_to_list() {
        // Couldn't add the node. Release the slot we've claimed.
        let node_ptr = &*node as *const Node as *mut Node;
        unsafe { ptr::drop_in_place(node_ptr); }
        master.unuse_node(node_ptr);

        UNUSED_NODE_SLOTS.fetch_add(1, Ordering::Release);
        return Err(AllocError);
    }

    Ok(Allocation { node })
}

// Claims a node slot. If that won't leave enough slots for every visitor to get one with the
// worst-case number of visitors, this function allocates a new master block, using the claimed
// slot, and repeats until there are enough slots.
fn alloc_masters(map: &MemoryMap) -> Result<(), AllocError> {
    let master_block_size = size_of::<MasterBlock>();
    let master_block_align = align_of::<MasterBlock>();

    update_expected_malloc_size(master_block_size);

    while UNUSED_NODE_SLOTS.load(Ordering::Acquire) <= MAX_VISITORS {
        // Loop to handle race conditions where multiple threads try to allocate the same memory at
        // the same time.
        let allocation = loop {
            let base = find_best_base(master_block_size, master_block_align, map)?;

            if let Ok(allocation) = allocate(base, NonZeroUsize::new(master_block_size).unwrap()) {
                break allocation;
            }
        };

        // Initialize the block of memory as a master block.
        unsafe {
            let master_block_ptr = PhysPtr::<_, *mut MasterBlock>::from_addr_phys(allocation.node.base()).as_virt().unwrap();
            master_block_ptr.write(MasterBlock::new(Some(allocation)));
            (*master_block_ptr).allocation().as_ref().unwrap().node.set_is_master(true, Ordering::Release);
        }

        // Now that the master block is initialized, its nodes are available.
        UNUSED_NODE_SLOTS.fetch_add(NODES_PER_MASTER_BLOCK, Ordering::Release);
    }

    Ok(())
}

// Searches for and atomically claims a slot for a new node. Calling this function without first
// decrementing `UNUSED_NODE_SLOTS` runs the risk of a deadlock, so make sure to do that first.
// Returns `(node, node_master)`, where
//     `node` is a reference to the `Cell` containing the uninitialized node, and
//     `node_master` is a reference to the `MasterBlock` containing that cell.
fn claim_slot() -> (&'static Cell<MaybeUninit<Node>>, &'static MasterBlock) {
    // It's possible that we'll go through the whole list without finding any free slots. But
    // in that case, we can still guarantee that there are free slots somewhere in the list, so
    // we'll find them eventually if we keep starting over.
    'restart: loop {
        // We'll check the static master block first, then each master block in the list.
        let mut master_block = &STATIC_MASTER_BLOCK;
        let mut nodes = nodes();

        let mut cells_masters = [None];
        let mut next_cell_master_index = 0;
        loop {
            // Claim slots in this master block as long as there are some available.
            loop {
                match master_block.claim_node() {
                    Some(cell) => {
                        cells_masters[next_cell_master_index] = Some((cell, master_block));
                        next_cell_master_index += 1;
                    },
                    None => break
                }

                if next_cell_master_index >= cells_masters.len() {
                    // We've claimed both slots.
                    return cells_masters[0].unwrap();
                }
            }

            // Find the next master block.
            loop {
                match nodes.next() {
                    Some(node) if node.is_master(Ordering::Acquire) => {
                        master_block = unsafe { &*PhysPtr::<_, *const _>::from_addr_phys(node.base()).as_virt().unwrap() };
                        break;
                    },
                    Some(_) => {},
                    None => continue 'restart   // No more master blocks. Try the whole list again.
                }
            }
        }
    }
}

// Rounds the given address up until it's aligned at a multiple of `align`.
fn align_up(addr: usize, align: usize) -> usize {
    assert!(align > 0);
    addr.wrapping_add(align - 1) / align * align
}

// Returns an iterator over all the nodes in the heap.
fn nodes() -> impl Iterator<Item = NodeRef> {
    Nodes::new()
}

// Represents a piece of memory that has been allocated on the heap. This object isn't used for
// much, but dropping it causes the memory to be freed.
#[derive(Debug)]
pub(crate) struct Allocation {
    node: Pin<&'static Node>
}

impl Drop for Allocation {
    fn drop(&mut self) {
        self.node.free();
    }
}


#[cfg(test)]
mod tests {
    use {
        super::*,
        core::{
            convert::TryInto,
            num::NonZeroUsize
        },
        std::{
            thread,
            time::{Duration, SystemTime}
        },
        oorandom::Rand64,
        spin::RwLock,
        crate::phys::map::{MemoryMap, RegionType}
    };

    // This lock ensures that, when necessary, only one test is manipulating the heap at a time.
    // If any test doesn't care about that, it should acquire a read lock, but those that do should
    // acquire the write lock.
    static HEAP_LOCK: RwLock<()> = RwLock::new(());
    
    const MEM_SIZE:    usize          = 0x100000;
    static mut MEMORY: [u8; MEM_SIZE] = [42u8; MEM_SIZE];

    lazy_static! {
        unsafe {
            static ref MAP: MemoryMap = memory_map(&*(&MEMORY[ .. ] as *const _ as *const [u8]));
        }
    }

    mod malloc {
        use super::*;

        #[test]
        fn should_use_correct_alignment() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(50).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (ptr1, allocation1) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation1, Some(&*MAP));
            validate_heap();
            let (ptr2, allocation2) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation2, Some(&*MAP));
            validate_heap();
            assert_eq!(ptr1.as_addr_phys() % align.get(), 0);
            assert_eq!(ptr2.as_addr_phys() % align.get(), 0);
            drop(allocation1);
            validate_heap();
            drop(allocation2);
            validate_heap();
        }
    }

    mod malloc_then_drop {
        use super::*;

        #[test]
        fn one_time_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            assert_heap_empty();
        }

        #[test]
        fn two_times_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            assert_heap_empty();
            let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            assert_heap_empty();
        }

        #[test]
        fn three_times_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            assert_heap_empty();
            let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            assert_heap_empty();
            let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            assert_heap_empty();
        }

        #[test]
        fn up_to_a_hundred_times_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            for _ in 0 .. 100 {
                let (_, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
                validate_allocation(&allocation, Some(&*MAP));
                validate_heap();
                drop(allocation);
                validate_heap();
                assert_heap_empty();
            }
        }
    }

    mod malloc_then_dealloc {
        use super::*;

        #[test]
        fn one_time_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            mem::forget(allocation);
            dealloc(ptr.into());
            validate_heap();
            assert_heap_empty();
        }

        #[test]
        fn two_times_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            mem::forget(allocation);
            dealloc(ptr.into());
            validate_heap();
            assert_heap_empty();
            let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            mem::forget(allocation);
            dealloc(ptr.into());
            validate_heap();
            assert_heap_empty();
        }

        #[test]
        fn three_times_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            mem::forget(allocation);
            dealloc(ptr.into());
            validate_heap();
            assert_heap_empty();
            let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            mem::forget(allocation);
            dealloc(ptr.into());
            validate_heap();
            assert_heap_empty();
            let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            mem::forget(allocation);
            dealloc(ptr.into());
            validate_heap();
            assert_heap_empty();
        }

        #[test]
        fn up_to_a_hundred_times_should_return_to_empty() {
            let size = NonZeroUsize::new(1).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            for _ in 0 .. 100 {
                let (ptr, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
                validate_allocation(&allocation, Some(&*MAP));
                mem::forget(allocation);
                dealloc(ptr.into());
                validate_heap();
                assert_heap_empty();
            }
        }
    }

    mod malloc_drop_malloc {
        use super::*;

        #[test]
        fn should_return_same_address() {
            let size = NonZeroUsize::new(100).unwrap();
            let align = NonZeroUsize::new(1).unwrap();
            let _lock = HEAP_LOCK.write();
            clear_heap();
            let (ptr1, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            drop(allocation);
            validate_heap();
            let (ptr2, allocation) = malloc_with_map(size, align, &*MAP).unwrap();
            validate_allocation(&allocation, Some(&*MAP));
            validate_heap();
            assert_eq!(ptr1, ptr2);
            drop(allocation);
            validate_heap();
        }
    }

    mod many_threads {
        use super::*;

        #[test]
        fn should_keep_the_heap_valid() {
            const THREADS_COUNT:  usize = 50;
            assert!(THREADS_COUNT > MAX_VISITORS);
            const OPS_PER_THREAD: usize = 1000;

            let static_master_base = PhysPtr::<_, *const _>::from_virt(&STATIC_MASTER_BLOCK).as_addr_phys();
            let static_master_end  = static_master_base + mem::size_of_val(&STATIC_MASTER_BLOCK);
            println!(
                "Static master block: [{:#010x}_{:08x}, {:#010x}_{:08x})",
                static_master_base >> 32, static_master_base & ((1 << 32) - 1),
                static_master_end >> 32, static_master_end & ((1 << 32) - 1)
            );

            // Initialize the map now so we get proper output on stdout.
            let _ = &*MAP;

            // Set up the pseudorandom number generator.
            let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                .expect("SystemTime::now() claims to be before the UNIX epoch")
                .as_nanos();
            println!("PRNG seed: {}", seed);
            let mut rng = Rand64::new(seed);

            // Decide on the operations each thread will perform.
            let mut operations = Vec::with_capacity(THREADS_COUNT);
            for _ in 0 .. THREADS_COUNT {
                let mut thread_ops = Vec::with_capacity(OPS_PER_THREAD);
                for _ in 0 .. OPS_PER_THREAD {
                    thread_ops.push(HeapOperation::new_random(&mut rng, MEM_SIZE.try_into().unwrap()));
                }
                operations.push(thread_ops);
            }

            let _lock = HEAP_LOCK.write();
            clear_heap();

            // Launch the threads and wait for them to finish.
            let mut child_threads = Vec::with_capacity(THREADS_COUNT);
            for thread_ops in operations.drain( .. ) {
                child_threads.push(thread::spawn((|ops: Vec<HeapOperation>| {
                    || {
                        let mut allocations = Vec::new();

                        for op in ops {
                            match op {
                                HeapOperation::Malloc { size, align } => {
                                    if let Ok((_base, allocation)) = malloc_with_map(
                                        NonZeroUsize::new(size).unwrap(),
                                        NonZeroUsize::new(align).unwrap(),
                                        &*MAP
                                    ) {
                                        validate_allocation(&allocation, Some(&*MAP));
                                        allocations.push(allocation);
                                    }
                                },
                                HeapOperation::ReserveMmio { base, size } => {
                                    if let Ok(allocation) = reserve_mmio_with_map(
                                        base,
                                        NonZeroUsize::new(size).unwrap(),
                                        &*MAP
                                    ) {
                                        validate_allocation(&allocation, None);
                                        allocations.push(allocation);
                                    }
                                },
                                HeapOperation::DropAllocated { index } => {
                                    if allocations.len() > 0 {
                                        drop(allocations.swap_remove(index % allocations.len()));
                                    }
                                },
                                HeapOperation::Dealloc { index } => {
                                    // This does the same thing as `DropAllocated`, but it uses `dealloc`
                                    // instead of just dropping the `Allocation` instance.
                                    if allocations.len() > 0 {
                                        let allocation = allocations.swap_remove(index % allocations.len());
                                        let addr = allocation.node.base();
                                        mem::forget(allocation);
                                        dealloc(PhysPtr::<_, *mut _>::from_addr_phys(addr));
                                    }
                                },
                                HeapOperation::Sleep { duration } => {
                                    thread::sleep(duration);
                                }
                            };
                        }
                    }
                })(thread_ops)));
            }
            let mut panic_count = 0;
            for child in child_threads {
                if let Err(error) = child.join() {
                    if let Some(message) = error.downcast_ref::<&str>() {
                        println!("panic occurred in a child thread: {}", message);
                    } else if let Some(message) = error.downcast_ref::<String>() {
                        println!("panic occurred in a child thread: {}", message);
                    } else {
                        println!("panic occurred in a child thread");
                    }
                    panic_count += 1;
                }
            }
            assert!(panic_count == 0, "number of panicking child threads: {}", panic_count);

            // Make sure the heap is still valid after all that chaos.
            validate_heap();
        }
        
        enum HeapOperation {
            Malloc {
                size:  usize,
                align: usize
            },
            ReserveMmio {
                base: PhysPtr<u8, *mut u8>,
                size: usize
            },
            DropAllocated {
                index: usize
            },
            Dealloc {
                index: usize
            },
            Sleep {
                duration: Duration
            }
        }

        impl HeapOperation {
            fn new_random(rng: &mut Rand64, mem_size: u64) -> Self {
                match rng.rand_range(0 .. 5) {
                    0 => Self::Malloc {
                        size:  rng.rand_range(1 .. mem_size) as usize,
                        align: rng.rand_range(1 .. mem_size / 2) as usize
                    },
                    1 => Self::ReserveMmio {
                        base: PhysPtr::<_, *mut _>::from_addr_phys(rng.rand_u64() as usize),
                        size: rng.rand_range(1 .. mem_size) as usize
                    },
                    2 => Self::DropAllocated {
                        index: rng.rand_u64() as usize
                    },
                    3 => Self::Dealloc {
                        index: rng.rand_u64() as usize
                    },
                    4 => Self::Sleep {
                        duration: Duration::from_millis(rng.rand_range(0 .. 3))
                    },
                    _ => unreachable!()
                }
            }
        }

        unsafe impl Send for HeapOperation {}
        unsafe impl Sync for HeapOperation {}
    }

    fn memory_map(memory: &[u8]) -> MemoryMap {
        let mut map = MemoryMap::new();
        let base = PhysPtr::<_, *const _>::from_virt(memory as *const _ as *const u8).as_addr_phys();
        let size = mem::size_of_val(memory);
        println!("Memory region: [{:#010x}_{:08x}, {:#010x}_{:08x})",
            base >> 32, base & 0xffff_ffff,
            (base + size) >> 32, (base + size) & 0xffff_ffff);
        map.add_region(base, NonZeroUsize::new(size).unwrap(), RegionType::Ram, false)
            .expect("failed to add a region to the memory map");
        map
    }

    fn clear_heap() {
        // Deallocate every node in the heap. This really only sets a flag in each node; they're
        // not removed from the list until the next time we iterate over them.
        for node in nodes() {
            node.free();
        }

        // Repeatedly iterate through the whole heap until the nodes are actually gone.
        while nodes().count() > 0 {}
    }

    fn validate_heap() {
        // Make sure that the nodes are sorted by base address and don't overlap. This check also
        // catches any situation in which the list of nodes contains a cycle, since that would cause
        // a jump to an earlier node.
        let (mut prev_base, mut prev_size) = (0, 0);
        for node in nodes() {
            let base = node.base();
            let size = node.size();
            println!("Validating node @ {:#p}: [{:#010x}_{:08x}, {:#010x}_{:08x})",
                &**node as *const Node,
                base >> 32, base & 0xffff_ffff,
                (base + size) >> 32, (base + size) & 0xffff_ffff);
            assert!(
                base >= prev_base + prev_size,
                "({:#x}, {:#x}) is after ({:#x}, {:#x}) but should be before it", base, size, prev_base, prev_size
            );
            prev_base = base;
            prev_size = size;
        }
    }

    fn assert_heap_empty() {
        // Make sure that the only nodes in the heap point to master blocks.
        for node in nodes() {
            assert!(
                node.is_master(Ordering::Acquire) || node.freeing(Ordering::Acquire),
                "expected empty heap, found node: {:#x?}", *node
            );
        }
    }

    fn validate_allocation(alloc: &Allocation, map: Option<&MemoryMap>) {
        // Make sure that `alloc.node` is actually in the heap's list and that `*alloc.node`
        // is entirely contained within an allocated master block.
        let (mut node_in_list, mut node_in_master) = (false, false);
        let alloc_base = PhysPtr::<_, *const _>::from_virt(&*alloc.node).as_addr_phys();
        let alloc_size = mem::size_of_val(&*alloc.node);
        for node in nodes() {
            let node_base = node.base();
            let node_size = node.size();

            if ptr::eq(&**node, &*alloc.node) {
                node_in_list = true;
                assert!(!node.is_master(Ordering::Acquire), "publicly visible nodes should not be master nodes");
            } else if !node_in_master && node.is_master(Ordering::Acquire) && node_base <= alloc_base
                    && alloc_base - node_base + alloc_size <= node_size {   // Rearranged from b + s <= b + s to avoid overflow
                node_in_master = true;
            }

            if node_in_list && node_in_master {
                break;
            }
        }
        // The static master block isn't in the heap's list, so we have to check for it separately.
        let static_master_base = PhysPtr::<_, *const _>::from_virt(&STATIC_MASTER_BLOCK).as_addr_phys();
        let static_master_size = mem::size_of_val(&STATIC_MASTER_BLOCK);
        let node_in_static_master = static_master_base <= alloc_base
            && alloc_base - static_master_base + alloc_size <= static_master_size;
        node_in_master = node_in_master || node_in_static_master;
        assert!(node_in_list, "the given node @ {:#p} is not in the heap's list", alloc.node);
        assert!(node_in_master, "the given node @ {:#p} is not in a master block", alloc.node);

        if let Some(map) = map {
            if !node_in_static_master {
                // Make sure that `alloc.node` is contained within the memory map.
                let node_in_region = map.present_regions()
                    .find(|region| region.base <= alloc_base
                        && alloc_base - region.base + alloc_size <= region.size) // Rearranged from b + s <= b + s to avoid overflow
                    .is_some();
                assert!(node_in_region, "the given node @ {:#p} is not in any of the memory map's regions", alloc.node);
            }
        }
    }
}
