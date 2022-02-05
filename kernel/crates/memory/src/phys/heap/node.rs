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

//! This module defines "nodes" that are used to represent allocated blocks in the kernel's heap.
//! These nodes are designed to be strung together in a lock-free singly linked list.

use {
    alloc::alloc::AllocError,
    core::{
        cell::{Cell, RefCell},
        fmt,
        hint,
        marker::PhantomPinned,
        mem::{self, MaybeUninit, size_of},
        num::NonZeroUsize,
        ops::Deref,
        pin::Pin,
        ptr,
        sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}
    },

    i18n::Text,
    tagged_ptr::TaggedPtr,

    super::Allocation
};

// The amount to add to a pointer's tag to indicate a change in the number of references to a node.
const GENERATION_STEP: usize = TaggedPtr::<Node>::TAG_UNIT;

// Either a node that defines a block or a guard between block nodes.
#[derive(Debug)]
#[repr(align(64))]
pub(crate) struct Node {
    // A pointer to the next node in the list.
    next: TaggedPtr<Node>,

    // A number that changes every time a reference to this node is dropped. If it is equal to the
    // similar number stored in the tag of the `TaggedPtr` that points to this node, there are no
    // references to it except that `TaggedPtr` itself.
    dropped_refs_gen: AtomicUsize,

    // A reference to the master block that contains this node. Because of a quirk with how we
    // initialize the heap, `None` means the statically allocated master block.
    master: Option<&'static MasterBlock>,

    // Extra contents that are necessary for block nodes but not for guards.
    block_node_contents: Option<BlockNodeContents>,

    _pin: PhantomPinned
}

impl Node {
    pub(crate) fn new_block_node(next: Pin<&Node>, base: usize, size: NonZeroUsize, master: &'static MasterBlock) -> Self {
        Self {
            next:                TaggedPtr::new(&*next as *const _ as *mut _, 0),
            dropped_refs_gen:    AtomicUsize::new(0),
            master:              Some(master),
            block_node_contents: Some(BlockNodeContents {
                base,
                size,
                freeing:          AtomicBool::new(false),
                is_master:        AtomicBool::new(false)
            }),
            _pin: PhantomPinned
        }
    }

    pub(crate) const fn new_guard(master: Option<&'static MasterBlock>) -> Self {
        Self {
            next:                TaggedPtr::new_null(0),
            dropped_refs_gen:    AtomicUsize::new(0),
            master,
            block_node_contents: None,
            _pin: PhantomPinned
        }
    }

    fn master(&self) -> &'static MasterBlock {
        self.master.unwrap_or(&super::STATIC_MASTER_BLOCK)
    }

    pub(crate) fn is_block_node(&self) -> bool {
        self.block_node_contents.is_some()
    }

    pub(crate) fn base(&self) -> Option<usize> {
        self.block_node_contents.as_ref().map(|contents| contents.base)
    }

    pub(crate) fn size(&self) -> Option<usize> {
        self.block_node_contents.as_ref().map(|contents| contents.size.get())
    }

    pub(crate) fn is_master(&self, ordering: Ordering) -> bool {
        self.block_node_contents.as_ref()
            .map(|c| c.is_master.load(ordering))
            .unwrap_or(false)
    }

    pub(crate) fn set_is_master(&self, value: bool, ordering: Ordering) {
        self.block_node_contents.as_ref()
            .expect("attempted to mark a guard as a master node")
            .is_master.store(value, ordering);
    }

    #[cfg(test)]
    pub(crate) fn freeing(&self, ordering: Ordering) -> bool {
        self.block_node_contents.as_ref()
            .map(|c| c.freeing.load(ordering))
            .unwrap_or(false)
    }

    pub(crate) fn next(&self) -> &TaggedPtr<Node> {
        &self.next
    }

    // Lazily frees this node. (This is currently needed only for block nodes.)
    pub(crate) fn free(&self) {
        if let Some(ref contents) = self.block_node_contents {
            // Since we don't know where the guard that points to this node is, we can't efficiently
            // remove this node from the list yet. But we can mark it so that it will be removed
            // whenever we happen to come across it in the future.
            contents.freeing.store(true, Ordering::Release);
        }
    }

    // Adds this block node, which must already be pointing to a guard that is also not yet in the list,
    // to the given heap's list of nodes and returns a reference to the guard immediately before it.
    pub(crate) fn add_to_list(self: Pin<&Self>) -> Result<NodeRef, AllocError> {
        assert!(self.is_block_node());
        super::check_heap_invariants("Node::add_to_list");

        let new_guard = NodeRef::from_tagged_ptr(&self.next).0
            .expect(Text::unexpected_end_of_heap());
        assert!(!new_guard.is_block_node());
        let mut old_guard = NodeRef::from_tagged_ptr(&super::first_guard_ptr()).0
            .expect(Text::unexpected_end_of_heap());
        assert!(!old_guard.is_block_node());
        loop {
            // Determine where the node should be added.
            let (temp_guard, (_, old_next_ptr)) = self.find_insert_location(old_guard)?;
            old_guard = temp_guard;

            // Set the forward pointer from the new guard, which still won't be in the list yet.
            new_guard.next.store(old_next_ptr, Ordering::Release);

            // Set the forward pointer from the old guard, inserting this node into the list.
            match old_guard.next.compare_exchange(
                    old_next_ptr,
                    (&*self as *const Node as *mut Node, 0),
                    Ordering::AcqRel,
                    Ordering::Acquire) {
                Ok(_)  => break,
                Err(_) => super::check_heap_invariants("Node::add_to_list->compare_exchange_failure")
            };
        }
        super::check_heap_invariants("Node::add_to_list->end");
        Ok(old_guard)
    }

    // Finds the guard after which this block node should be added. Also returns a reference to the
    // node currently following that guard (if any) and a snapshot of the tagged pointer from the
    // guard to that node.
    fn find_insert_location(&self, first_guard: NodeRef)
            -> Result<(NodeRef, (Option<NodeRef>, (*mut Node, usize))), AllocError> {
        assert!(self.is_block_node());

        let base = self.base().unwrap();
        let mut guard = first_guard;
        let mut block_node: NodeRef;
        let mut block_node_ptr: (*mut Node, usize);
        loop {
            // We must never put a new block node before an existing guard, so we need to put it
            // after a guard that's pointing to a block node or at the end of the list.
            loop {
                match NodeRef::from_tagged_ptr(&guard.next) {
                    (None, tagged_ptr) => {
                        // We've found the end of the list, so insert the new node here.
                        return Ok((guard, (None, tagged_ptr)));
                    },
                    (Some(node_ref), tagged_ptr) if node_ref.is_block_node() => {
                        block_node = node_ref;
                        block_node_ptr = tagged_ptr;
                        break;
                    },
                    (Some(guard_ref), _) => {
                        guard = guard_ref;
                    }
                };
            }

            // Keep traversing the list until we find a block that's after the given base.
            if block_node.base().unwrap() >= base {
                // Found one. Continue allocating only if there won't be any overlap.
                if block_node.base().unwrap() - base < self.size().unwrap() {
                    return Err(AllocError);
                }
                break;
            }

            guard = NodeRef::from_tagged_ptr(&block_node.next).0
                .expect(Text::unexpected_end_of_heap());
            assert!(!guard.is_block_node());
        }

        Ok((guard, (Some(block_node), block_node_ptr)))
    }
}

// The parts of a block node that are not also present in a guard.
#[derive(Debug)]
pub(crate) struct BlockNodeContents {
    // The base address of the block.
    base: usize,

    // The number of bytes of memory in the block.
    size: NonZeroUsize,

    // True if the block is in the process of being freed.
    freeing: AtomicBool,

    // True if this node controls a master block (used as space for more nodes).
    pub(crate) is_master: AtomicBool
}

// A reference to a node. An instance of this type should *never* be made directly, since that would
// bypass the reference counting. Instead, use `NodeRef::from_tagged_ptr`.
#[derive(Debug)]
pub(crate) struct NodeRef(Pin<&'static Node>);

impl NodeRef {
    // Reads a tagged pointer, increases its reference count, and returns a reference to the node it
    // points to, along with a snapshot of the tagged pointer.
    pub(crate) fn from_tagged_ptr(tagged_ptr: &TaggedPtr<Node>) -> (Option<Self>, (*mut Node, usize)) {
        let (ptr, tag) = tagged_ptr.fetch_add_tag(GENERATION_STEP, Ordering::AcqRel);
        let node_ref = if ptr.is_null() {
            None
        } else {
            Some(Self(unsafe { Pin::new_unchecked(&*ptr) }))
        };

        (node_ref, (ptr, tag.wrapping_add(GENERATION_STEP)))
    }

    // Tries once to remove this node from the list. This function returns `Err` if there are more
    // than one reference to this node (it would be undefined behavior to remove it while someone
    // else is using it).
    // 
    // If this function returns `Ok`, the node has been dropped. Any and all references and
    // pointers to it are dangling. As such, using a reference to this node in any way is undefined
    // behavior, as is dereferencing any pointer to this node.
    // 
    // Returns `Err(self)` if the removal fails, in case the reference is still needed.
    fn try_remove_from_list(self, earlier_ptr: &TaggedPtr<Node>) -> Result<(), Self> {
        super::check_heap_invariants("NodeRef::try_remove_from_list");

        // Find the tagged pointer that points to this guard.
        let mut prev_ptr = earlier_ptr;
        let mut prev_ref;
        let (mut old_ptr, mut old_new_refs_gen);
        loop {
            let node_ref;
            match NodeRef::from_tagged_ptr(prev_ptr) {
                (n, (p, t)) => {
                    node_ref = n;
                    old_ptr = p;
                    old_new_refs_gen = t;
                }
            };
            match node_ref {
                None => panic!("expected an earlier node when removing a node from the heap"),
                Some(node_ref) => {
                    if ptr::eq(&**node_ref, &**self) {
                        break;
                    }
                    prev_ref = node_ref;
                    prev_ptr = &prev_ref.next;
                }
            };
        }

        // Make sure the reference we have (`self`) is the only reference to this node.
        if old_new_refs_gen != self.dropped_refs_gen.load(Ordering::SeqCst).wrapping_add(GENERATION_STEP) {
            return Err(self);
        }

        // Remove the node from the list.
        // Relaxed ordering: If this CAS fails, we don't care what value we've read. Spurrious
        //     failures are also acceptable.
        if let Err(_) = prev_ptr.compare_exchange(
            (old_ptr, old_new_refs_gen),
            self.next.load(Ordering::Acquire),
            Ordering::SeqCst,
            Ordering::Relaxed
        ) {
            return Err(self);
        }

        // We should still have the only reference to this node.
        debug_assert_eq!(old_new_refs_gen, self.dropped_refs_gen.load(Ordering::SeqCst).wrapping_add(GENERATION_STEP));

        super::check_heap_invariants("NodeRef::try_remove_from_list->compare_exchange");

        // Drop the node and mark the slot it was using as unused.
        let master = self.master();
        let self_ptr = &**self as *const _ as *mut _;
        mem::forget(self);
        unsafe { ptr::drop_in_place(self_ptr); }
        master.unuse_node(self_ptr);

        super::check_heap_invariants("NodeRef::try_remove_from_list");

        Ok(())
    }
}

impl Deref for NodeRef {
    type Target = Pin<&'static Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for NodeRef {
    fn drop(&mut self) {
        self.dropped_refs_gen.fetch_add(GENERATION_STEP, Ordering::Release);
    }
}

/// An iterator over the block nodes in a heap.
pub(crate) struct BlockNodes {
    prev_guard:    Option<NodeRef>,
    current_guard: NodeRef
}

impl BlockNodes {
    pub(crate) fn new() -> Self {
        // Block until there are few enough simultaneous visitors to guarantee that the reference
        // counts won't overflow.
        // Relaxed ordering: We're not depending on the first value we read here being correct. If
        //     it's not, either the CAS loop will catch that mistake or we'll just retry this load.
        'outer: loop {
            let mut visitors = super::VISITORS.load(Ordering::Relaxed);
            loop {
                if visitors >= super::MAX_VISITORS {
                    hint::spin_loop();
                    break;
                }
                
                // Relaxed ordering: Same as above. On the failure path, we don't have to read the
                //     correct value because the loop is about to repeat, resulting in another CAS that
                //     will catch the mistake.
                match super::VISITORS.compare_exchange_weak(visitors, visitors + 1, Ordering::AcqRel, Ordering::Relaxed) {
                    Ok(_) => break 'outer,
                    Err(x) => visitors = x
                };
                hint::spin_loop();
            }
        }

        Self {
            prev_guard:    None,
            current_guard: NodeRef::from_tagged_ptr(&super::first_guard_ptr()).0
                .expect(Text::heap_contains_no_guards())
        }
    }
}

impl Iterator for BlockNodes {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next_node = match NodeRef::from_tagged_ptr(&self.current_guard.next).0 {
                None => {
                    // We've reached the end of the list.
                    return None;
                },
                Some(node_ref) => node_ref
            };

            match next_node.block_node_contents {
                None => {
                    // Found another guard. Remove the earlier one from the list if possible, then
                    // skip it. (Removing the later one would break the lock-free algorithm.)
                    let obsolete_guard = mem::replace(&mut self.current_guard, next_node);
                    let _ = obsolete_guard.try_remove_from_list(
                        self.prev_guard.as_ref()
                            .map(|guard| &guard.next)
                            .unwrap_or_else(|| super::first_guard_ptr())
                    );
                },
                Some(ref contents) if !contents.freeing.load(Ordering::Acquire) => {
                    // Found a block node.
                    self.prev_guard = Some(mem::replace(
                        &mut self.current_guard,
                        NodeRef::from_tagged_ptr(&next_node.next).0
                            .expect(Text::heap_block_node_not_followed_by_guard())
                    ));
                    assert!(!self.current_guard.is_block_node());
                    return Some(next_node);
                },
                Some(_) => {
                    // Found a block node, but it's supposed to be freed. Remove it from the list if
                    // possible.
                    let next_guard = NodeRef::from_tagged_ptr(&next_node.next).0
                        .expect(Text::heap_block_node_not_followed_by_guard());
                    self.prev_guard = Some(mem::replace(&mut self.current_guard, next_guard));
                    assert!(!self.current_guard.is_block_node());
                    if let Err(next_node) = next_node.try_remove_from_list(&self.prev_guard.as_ref().unwrap().next) {
                        // Removal failed, so this node will still exist for a while. We'll have to return it.
                        return Some(next_node);
                    }
                }
            };
        }
    }
}

impl Drop for BlockNodes {
    fn drop(&mut self) {
        // We are no longer visiting the heap's nodes.
        super::VISITORS.fetch_sub(1, Ordering::Release);
    }
}

// The number of nodes a master block can contain.
const NODES_PER_MASTER_BLOCK: usize = 64;

// The average number of guards that can be allocated in each master block.
pub(crate) const GUARDS_PER_MASTER_BLOCK: usize = NODES_PER_MASTER_BLOCK / 2;

// This struct is a block that contains the nodes that define the heap.
pub(crate) struct MasterBlock {
    allocation: RefCell<Option<Allocation>>, // A reference to the this block's heap allocation, if any
    nodes: [Cell<MaybeUninit<Node>>; NODES_PER_MASTER_BLOCK],
    nodes_used: AtomicU64,                // A bitmap of which elements of the `nodes` array currently exist

    formatting_debug: AtomicBool          // True if `Debug::fmt` is currently being called on this object
}

impl MasterBlock {
    // Returns a new statically allocated master block, which is intended to be created at compile
    // time. This master block contains, from its creation, the heap nodes needed to have the heap
    // in a valid state, ready to malloc.
    pub(crate) const fn make_first() -> Self {
        // TODO: Replace this with a simple array of `Cell`s.
        assert!(mem::size_of::<Cell<MaybeUninit<Node>>>() == mem::size_of::<MaybeUninit<Node>>());
        union Nodes {
            cells:    [Cell<MaybeUninit<Node>>; NODES_PER_MASTER_BLOCK],
            no_cells: [MaybeUninit<Node>;       NODES_PER_MASTER_BLOCK]
        }
        let mut nodes = Nodes { cells: array![Cell::new(MaybeUninit::uninit()); 64] };

        let mut nodes_used = 0;
        let index = 0;

        // Add a tail guard. The list of nodes should always end with one.
        // TODO: This simpler line should work, but it doesn't yet. (See https://github.com/rust-lang/rust/issues/69908 for current status.)
        //       nodes[index].set(MaybeUninit::new(Node::new_guard(None)));
        unsafe {
            nodes.no_cells[index] = MaybeUninit::new(Node::new_guard(None));
        }
        nodes_used |= 1 << index;
        //index += 1;

        Self {
            allocation:       RefCell::new(None),
            nodes:            unsafe { nodes.cells },
            nodes_used:       AtomicU64::new(nodes_used),
            formatting_debug: AtomicBool::new(false)
        }
    }

    // The number of slots available in the first master block for guard nodes right after its
    // `const` initialization if nodes are added to this block in this order: block, guard, block,
    // guard, ...
    pub(crate) const INITIAL_UNUSED_GUARD_SLOTS: usize = (NODES_PER_MASTER_BLOCK - 1) / 2;

    // Tries to return a pointer to the first guard node in this master block.
    // 
    // This function is designed to be used only once, as part of the heap's initialization. Any
    // other use is still defined behavior according to Rust, but it's undefined according to this
    // API. The returned pointer might point to an uninitialized value or a block node in that case.
    pub(crate) fn initial_first_guard_ptr(&self) -> *mut Node {
        unsafe { (*self.nodes[0].as_ptr()).as_mut_ptr() }
    }

    pub(crate) const fn new_dynamic(allocation: Allocation) -> Self {
        Self {
            allocation:       RefCell::new(Some(allocation)),
            nodes:            array![Cell::new(MaybeUninit::uninit()); 64],
            nodes_used:       AtomicU64::new(0),
            formatting_debug: AtomicBool::new(false)
        }
    }

    pub(crate) fn allocation(&self) -> impl '_+Deref<Target = Option<Allocation>> {
        self.allocation.borrow()
    }

    pub(crate) fn claim_node(&self) -> Option<&Cell<MaybeUninit<Node>>> {
        let mut nodes_used = self.nodes_used.load(Ordering::Acquire);

        while nodes_used != u64::max_value() {
            let index = nodes_used.trailing_ones(); // Index of the first zero
            let mask = 1 << index;
            nodes_used = self.nodes_used.fetch_or(mask, Ordering::AcqRel);
            if nodes_used & mask == 0 {
                // No one else claimed this node first.
                return Some(&self.nodes[index as usize]);
            }
        }

        None
    }

    // Marks the given node as unused. Note that this function is intended to be used *after* the node
    // is dropped. Otherwise, another node could take its place before the old one is dropped. Since
    // this function never dereferences the given pointer, no undefined behavior results.
    pub(crate) fn unuse_node(&self, node_ptr: *const Node) {
        // Mark the node as unused.
        let node_addr = node_ptr as usize;
        let nodes_addr = &self.nodes[0] as *const _ as usize;
        assert!(node_addr >= nodes_addr);
        assert_eq!((node_addr - nodes_addr) % size_of::<MaybeUninit<Node>>(), 0);
        let idx = (node_addr - nodes_addr) / size_of::<MaybeUninit<Node>>();
        let nodes_used = self.nodes_used.fetch_and(!(1 << idx), Ordering::AcqRel);
        assert_eq!(nodes_used & (1 << idx), 1 << idx);
        let nodes_used = nodes_used & !(1 << idx);

        // Free the master block if it's no longer needed.
        if nodes_used == 0 {
            self.try_free();
        }
    }

    // Frees this master block if all of its node slots are unused and if freeing it would not
    // reduce the number of unused slots below the minimum. Does nothing if this is a statically
    // allocated block.
    fn try_free(&self) {
        if self.allocation.borrow().is_none() {
            // No parent node means this block isn't dynamically allocated, so it can't be dynamically freed.
            return;
        }

        // Claim as many node slots as this block has, failing if that would leave too few slots
        // available.
        // Relaxed ordering: If we read the wrong value here, either we will return without freeing
        //     this master block (even if we should actually free it) or we'll execute a CAS that
        //     will catch the mistake. Not freeing the master block isn't ideal, but the heap will
        //     still be in a valid state, and the master block will be reused for later allocations.
        let mut old_unused_guard_slots = super::UNUSED_GUARD_SLOTS.load(Ordering::Relaxed);
        loop {
            let new_unused_guard_slots = old_unused_guard_slots.saturating_sub(GUARDS_PER_MASTER_BLOCK);
            if new_unused_guard_slots < super::MAX_VISITORS {
                // There would be too few slots left if we continued.
                return;
            }

            // Relaxed ordering: Same as above. If we read the wrong value on the error path, we'll
            //     either return without freeing the master block or execute another CAS that will
            //     catch the mistake.
            match super::UNUSED_GUARD_SLOTS.compare_exchange_weak(
                    old_unused_guard_slots,
                    new_unused_guard_slots,
                    Ordering::AcqRel,
                    Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => old_unused_guard_slots = x
            };
        }

        // Actually mark the nodes in this master block as used so they won't be claimed by anyone
        // else.
        if self.nodes_used.compare_exchange(0, u64::max_value(), Ordering::AcqRel, Ordering::Acquire).is_err() {
            // Someone's already claimed a node. Undo everything we've done so far.
            super::UNUSED_GUARD_SLOTS.fetch_add(GUARDS_PER_MASTER_BLOCK, Ordering::Release);
            return;
        }

        // Now we can safely free this master block because we definitely have the only reference to
        // it now. (The heap only has a reference to its parent node.)
        self.allocation.replace(None);
    }
}

unsafe impl Sync for MasterBlock {}

impl fmt::Debug for MasterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Avoid infinite recursion due to the reference cycles we'll encounter. (N.B. This doesn't
        // work very well if multiple threads call this function at the same time. But since it's
        // supposed to be only for debugging output, I don't expect that to happen.)
        if self.formatting_debug.swap(true, Ordering::AcqRel) {
            return write!(f, "MasterBlock (reference cycle)");
        }

        let nodes_used = self.nodes_used.load(Ordering::Acquire);

        write!(f, "MasterBlock {{ allocation: {:?}, nodes: [", self.allocation)?;
        if nodes_used & 1 != 0 {
            write!(f, "{:?}", *unsafe { (*self.nodes[0].as_ptr()).assume_init_ref() })?;
        } else {
            write!(f, "<Uninit>")?;
        }
        for i in 1 .. self.nodes.len() {
            if nodes_used & (1 << i) != 0 {
                write!(f, ", {:?}", *unsafe { (*self.nodes[i].as_ptr()).assume_init_ref() })?;
            } else {
                write!(f, ", <Uninit>")?;
            }
        }
        write!(f, "], nodes_used: {:?} }}", self.nodes_used)?;

        self.formatting_debug.store(false, Ordering::Release);

        Ok(())
    }
}
