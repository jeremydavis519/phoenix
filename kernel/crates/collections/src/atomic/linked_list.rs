/* Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines an atomically accessible linked list.

use {
    alloc::boxed::Box,
    core::{
        cmp,
        mem,
        ops::Deref,
        sync::atomic::{AtomicUsize, Ordering}
    },
    locks::Semaphore,
    tagged_ptr::TaggedPtr
};

/// A linked list that can be accessed and modified atomically.
pub struct AtomicLinkedList<T, ListAlloc: alloc::alloc::Allocator+Copy = alloc::alloc::Global, ElemAlloc: alloc::alloc::Allocator = ListAlloc> {
    allocator: ListAlloc,
    head: TaggedPtr<Node<T, ElemAlloc>>
}

#[repr(align(32))]
struct Node<T, ElemAlloc: alloc::alloc::Allocator> {
    element: Option<Box<T, ElemAlloc>>, // This is always `Some` as long as the node is in the list.
    next: TaggedPtr<Node<T, ElemAlloc>>,
    ref_drop_counter: AtomicUsize
}

/// A reference to an element in an `AtomicLinkedList`. While this reference is held, the element
/// it refers to cannot be removed from the list.
pub struct ElemRef<'a, T: 'a, ListAlloc: 'a+alloc::alloc::Allocator+Copy, ElemAlloc: 'a+alloc::alloc::Allocator> {
    node: &'a Node<T, ElemAlloc>,
    list: &'a AtomicLinkedList<T, ListAlloc, ElemAlloc>
}

impl<T> AtomicLinkedList<T> {
    /// Creates a new empty `AtomicLinkedList` (behind a semaphore) using the global allocator.
    pub const fn new() -> Semaphore<AtomicLinkedList<T>> {
        AtomicLinkedList::with_allocator(alloc::alloc::Global)
    }
}

impl<T, ListAlloc: alloc::alloc::Allocator+Copy, ElemAlloc: alloc::alloc::Allocator> AtomicLinkedList<T, ListAlloc, ElemAlloc> {
    const TAG_UNIT: usize = TaggedPtr::<Node<T, ElemAlloc>>::TAG_UNIT;

    /// Creates a new empty `AtomicLinkedList` (behind a semaphore) using the given allocator.
    pub const fn with_allocator(allocator: ListAlloc) -> Semaphore<AtomicLinkedList<T, ListAlloc, ElemAlloc>> {
        // The limitation on the number of simultaneous visitors comes from the use of `TaggedPtr`
        // to keep track of simultaneous references to the same node.
        Semaphore::new(
            AtomicLinkedList {
                allocator,
                head: TaggedPtr::new_null(0)
            },
            mem::align_of::<Node<T, ElemAlloc>>() - 1
        )
    }

    /// Inserts `new_element` into the list at the head.
    ///
    /// # Returns
    /// `Ok(())` if the element is inserted successfully, else `Err(new_element)`. An error can
    /// occur if another visitor inserts or removes an element at the head before this function
    /// finishes.
    pub fn insert_head(&self, new_element: Box<T, ElemAlloc>) -> Result<(), Box<T, ElemAlloc>> {
        let (expected_ptr, expected_tag) = self.head.load(Ordering::Acquire);
        self.insert_after_ptr_with_expected(&self.head, new_element, expected_ptr, expected_tag)
    }

    /// Inserts `new_element` into the list right after `pre_element`.
    ///
    /// # Returns
    /// `Ok(())` if the element is inserted successfully, else `Err(new_element)`. An error can
    /// occur if another visitor inserts or removes an element right after `pre_element` before
    /// this function finishes.
    ///
    /// # Panics
    /// If `pre_element` did not come from this list.
    pub fn insert_after(
            &self,
            pre_element: &ElemRef<T, ListAlloc, ElemAlloc>,
            new_element: Box<T, ElemAlloc>
    ) -> Result<(), Box<T, ElemAlloc>> {
        assert_eq!(pre_element.list as *const _, self as *const _);

        let (expected_ptr, expected_tag) = pre_element.node.next.load(Ordering::Acquire);
        self.insert_after_ptr_with_expected(&pre_element.node.next, new_element, expected_ptr, expected_tag)
    }

    /// Inserts `new_element` into the list, maintaining the way in which the list has already been
    /// sorted. If the list is not sorted, this function is just a bad way to insert an element
    /// into a pseudorandom position in the list.
    ///
    /// # Returns
    /// `Ok(pre_element)` if the element is inserted successfully, else `Err((pre_element, new_element))`.
    /// An error can occur if another visitor inserts or removes an element right where we were
    /// going to insert `new_element` before this function finishes. In that case, consider calling
    /// `insert_sorted_after` to try again without traversing the entire list again.
    pub fn insert_sorted<F>(
            &self,
            new_element: Box<T, ElemAlloc>,
            compare: F
        ) -> Result<Option<ElemRef<T, ListAlloc, ElemAlloc>>, (Option<ElemRef<T, ListAlloc, ElemAlloc>>, Box<T, ElemAlloc>)>
            where F: Fn(&T, &T) -> cmp::Ordering {
        self.insert_sorted_after(None, new_element, compare)
    }

    /// Inserts `new_element` into the list, maintaining the way in which the list has already been
    /// sorted. If the list is not sorted, this function is just a bad way to insert an element
    /// into a pseudorandom position in the list.
    ///
    /// This function is meant to be used if `insert_sorted` fails, although it can also be used or
    /// if you already have a reference to an earlier node by iteration. It begins the linear search
    /// at the node following `pre_element` rather than at the head of the list.
    ///
    /// # Returns
    /// `Ok(pre_element)` if the element is inserted successfully, else `Err((pre_element_2, new_element))`.
    /// An error can occur if another visitor inserts or removes an element right where we were going
    /// to insert `new_element` before this function finishes. In that case, you can retry by calling
    /// `list.insert_sorted(pre_element_2, new_element, compare)`.
    ///
    /// # Panics
    /// If `pre_element` did not come from this list.
    pub fn insert_sorted_after<'a, F>(
            &'a self,
            pre_element: Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>,
            new_element: Box<T, ElemAlloc>,
            compare: F
        ) -> Result<Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>, (Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>, Box<T, ElemAlloc>)>
            where F: Fn(&T, &T) -> cmp::Ordering {
        let ptr = match pre_element {
            Some(ref elem) => {
                assert_eq!(elem.list as *const _, self as *const _);
                &elem.node.next
            },
            None => &self.head
        };

        self.insert_sorted_after_ptr(ptr, pre_element, new_element, compare)
    }

    // The implementation of `insert_sorted[_after]`.
    fn insert_sorted_after_ptr<'a, F>(
            &'a self,
            mut ptr: &'a TaggedPtr<Node<T, ElemAlloc>>,
            mut pre_element: Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>,
            new_element: Box<T, ElemAlloc>,
            compare: F
    ) -> Result<Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>, (Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>, Box<T, ElemAlloc>)>
            where F: Fn(&T, &T) -> cmp::Ordering {
        loop {
            match ElemRef::new(ptr, self) {
                (None, (expected_ptr, expected_tag)) => {
                    // End of the list. Try to insert here.
                    return match self.insert_after_ptr_with_expected(ptr, new_element, expected_ptr, expected_tag) {
                        Ok(()) => Ok(pre_element),
                        Err(new_element) => Err((pre_element, new_element))
                    };
                },
                (Some(elem), (expected_ptr, expected_tag)) => match compare(&*elem, &*new_element) {
                    cmp::Ordering::Less => {
                        // Keep searching.
                        ptr = &elem.node.next;
                        pre_element = Some(elem);
                    },
                    cmp::Ordering::Equal | cmp::Ordering::Greater => {
                        // This is the proper place to insert the new element.
                        return match self.insert_after_ptr_with_expected(ptr, new_element, expected_ptr, expected_tag) {
                            Ok(()) => Ok(pre_element),
                            Err(new_element) => Err((pre_element, new_element))
                        };
                    }
                }
            }
        }
    }

    // The ultimate implementation of `insert_*`.
    fn insert_after_ptr_with_expected(
            &self,
            ptr: &TaggedPtr<Node<T, ElemAlloc>>,
            new_element: Box<T, ElemAlloc>,
            expected_ptr: *mut Node<T, ElemAlloc>,
            expected_tag: usize
    ) -> Result<(), Box<T, ElemAlloc>> {
        let new_node = Box::new_in(Node {
            element: Some(new_element),
            next: TaggedPtr::new(expected_ptr, expected_tag),
            ref_drop_counter: AtomicUsize::new(0)
        }, self.allocator);

        let (new_ptr, allocator) = Box::into_raw_with_allocator(new_node);

        ptr.compare_exchange(
            (expected_ptr, expected_tag),
            (new_ptr, 0),
            Ordering::AcqRel,
            Ordering::Acquire
        )
            .map(|_| ())
            .map_err(|_| mem::replace(unsafe { &mut Box::from_raw_in(new_ptr, allocator).element }, None).unwrap())
    }

    /// Returns a reference to the first element of the list (or `None` if the list is empty).
    pub fn head(&self) -> Option<ElemRef<T, ListAlloc, ElemAlloc>> {
        self.iter().next()
    }

    /// Returns `true` if the list is empty.
    pub fn is_empty(&self) -> bool {
        let (ptr, _) = self.head.load(Ordering::Acquire);
        ptr.is_null()
    }

    /// Attempts to remove `old_element` from the head of the list.
    ///
    /// # Returns
    /// `Ok(b)` if successful (where `b` is the `Box` that was originally given to one of the
    /// `insert_*` methods), else `Err(old_element)`. An error can occur if another visitor inserts
    /// a new element at the head before this function finishes.
    ///
    /// # Panics
    /// If `old_element` did not come from this list.
    pub fn remove_head<'a>(&'a self, old_element: ElemRef<'a, T, ListAlloc, ElemAlloc>)
            -> Result<Box<T, ElemAlloc>, ElemRef<'a, T, ListAlloc, ElemAlloc>> {
        assert_eq!(old_element.list as *const _, self as *const _);

        self.remove_after_ptr(&self.head, old_element)
    }

    /// Attempts to remove `old_element` from the list.
    ///
    /// # Returns
    /// `Ok(b)` if successful (where `b` is the `Box` that was originally given to one of the
    /// `insert_*` methods), else `Err(old_element)`. An error can occur if `pre_element` is not
    /// immediately before `old_element`, for instance if another visitor inserts a new element
    /// between them.
    ///
    /// # Panics
    /// If either `pre_element` or `old_element` did not come from this list.
    pub fn remove_after<'a>(
            &'a self,
            pre_element: &'a ElemRef<'a, T, ListAlloc, ElemAlloc>,
            old_element: ElemRef<'a, T, ListAlloc, ElemAlloc>
    ) -> Result<Box<T, ElemAlloc>, ElemRef<'a, T, ListAlloc, ElemAlloc>> {
        assert_eq!(pre_element.list as *const _, self as *const _);
        assert_eq!(old_element.list as *const _, self as *const _);

        self.remove_after_ptr(&pre_element.node.next, old_element)
    }

    // The implementation of `remove_head` and `remove_after`.
    fn remove_after_ptr<'a>(&'a self, ptr: &'a TaggedPtr<Node<T, ElemAlloc>>, old_element: ElemRef<'a, T, ListAlloc, ElemAlloc>)
            -> Result<Box<T, ElemAlloc>, ElemRef<'a, T, ListAlloc, ElemAlloc>> {
        let expected_ptr = old_element.node as *const _ as *mut _;

        // We mustn't remove the element if the current number of references to it is not 1 (since
        // we should have the only reference).
        let expected_tag = old_element.node.ref_drop_counter.load(Ordering::Acquire).wrapping_add(Self::TAG_UNIT);

        // If we have the only reference to `old_element`, its `next` pointer can't be changed
        // between now and when we remove it.
        let (new_ptr, new_tag) = old_element.node.next.load(Ordering::Acquire);

        match ptr.compare_exchange(
            (expected_ptr, expected_tag),
            (new_ptr, new_tag),
            Ordering::AcqRel,
            Ordering::Acquire
        ) {
            Ok(_) => {
                let mut node = unsafe { Box::from_raw_in(expected_ptr, self.allocator) };
                Ok(mem::replace(&mut node.element, None).expect("node in AtomicLinkedList doesn't have an element"))
            },
            Err(_) => Err(old_element)
        }
    }

    /// Returns an iterator over all the elements in this list.
    ///
    /// Note that, since the list is lock-free, it might change at any time. The sequence of
    /// elements returned by this iterator might never be present all at the same time. What is
    /// guaranteed is that, as long a reference to an element is held, that element will not be
    /// destroyed or moved in memory, nor will two referenced elements change their relative order
    /// in the list. One consequence of this property is that it's normally impossible to know
    /// whether you have iterated over every element currently in the list. If it's necessary to
    /// know this, the whole list must be locked, for instance with a read-write lock.
    pub const fn iter(&self) -> AtomicLinkedListIter<T, ListAlloc, ElemAlloc> {
        AtomicLinkedListIter { list: self, next_ptr: &self.head }
    }
}

/// An iterator over all the elements in an `AtomicLinkedList`.
pub struct AtomicLinkedListIter<'a, T: 'a, ListAlloc: 'a+alloc::alloc::Allocator+Copy, ElemAlloc: 'a+alloc::alloc::Allocator> {
    list: &'a AtomicLinkedList<T, ListAlloc, ElemAlloc>,
    next_ptr: &'a TaggedPtr<Node<T, ElemAlloc>>
}

impl<'a, T: 'a, ListAlloc: 'a+alloc::alloc::Allocator+Copy, ElemAlloc: 'a+alloc::alloc::Allocator> Iterator
        for AtomicLinkedListIter<'a, T, ListAlloc, ElemAlloc> {
    type Item = ElemRef<'a, T, ListAlloc, ElemAlloc>;

    fn next(&mut self) -> Option<Self::Item> {
        match ElemRef::new(self.next_ptr, self.list) {
            (None, _) => None,
            (Some(elem_ref), _) => {
                self.next_ptr = &elem_ref.node.next;
                Some(elem_ref)
            }
        }
    }
}

/// Some methods don't require incrementing the tags on pointers, so they don't need the semaphore's
/// guarantee. Those methods are exposed here on the semaphore itself.
pub trait AtomicLinkedListSemaphore<T, ElemAlloc: alloc::alloc::Allocator> {
    /// Inserts `new_element` into the list at the head.
    ///
    /// # Returns
    /// `Ok(())` if the element is inserted successfully, else `Err(new_element)`. An error can
    /// occur if another visitor inserts or removes an element at the head before this function
    /// finishes.
    fn insert_head(&self, new_element: Box<T, ElemAlloc>) -> Result<(), Box<T, ElemAlloc>>;

    /// Returns `true` if the list is empty.
    fn is_empty(&self) -> bool;
}
impl<T, ListAlloc: alloc::alloc::Allocator+Copy, ElemAlloc: alloc::alloc::Allocator> AtomicLinkedListSemaphore<T, ElemAlloc>
        for Semaphore<AtomicLinkedList<T, ListAlloc, ElemAlloc>> {
    fn insert_head(&self, new_element: Box<T, ElemAlloc>) -> Result<(), Box<T, ElemAlloc>> {
        unsafe { self.force_access().insert_head(new_element) }
    }

    fn is_empty(&self) -> bool {
        unsafe { self.force_access().is_empty() }
    }
}

impl<'a, T: 'a, ListAlloc: 'a+alloc::alloc::Allocator+Copy, ElemAlloc: 'a+alloc::alloc::Allocator> ElemRef<'a, T, ListAlloc, ElemAlloc> {
    fn new(ptr: &TaggedPtr<Node<T, ElemAlloc>>, list: &'a AtomicLinkedList<T, ListAlloc, ElemAlloc>)
            -> (Option<ElemRef<'a, T, ListAlloc, ElemAlloc>>, (*mut Node<T, ElemAlloc>, usize)) {
        let (node, tag) = ptr.fetch_add_tag(TaggedPtr::<Node<T, ElemAlloc>>::TAG_UNIT, Ordering::AcqRel);
        let tag = tag.wrapping_add(TaggedPtr::<Node<T, ElemAlloc>>::TAG_UNIT);
        if node.is_null() {
            (None, (node, tag))
        } else {
            (
                Some(ElemRef {
                    node: unsafe { &*node },
                    list
                }),
                (node, tag)
            )
        }
    }
}

impl<'a, T: 'a, ListAlloc: 'a+alloc::alloc::Allocator+Copy, ElemAlloc: 'a+alloc::alloc::Allocator> Deref for ElemRef<'a, T, ListAlloc, ElemAlloc> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.node.element.as_ref().expect("node in AtomicLinkedList doesn't have an element")
    }
}

impl<'a, T: 'a, ListAlloc: 'a+alloc::alloc::Allocator+Copy, ElemAlloc: 'a+alloc::alloc::Allocator> Drop for ElemRef<'a, T, ListAlloc, ElemAlloc> {
    fn drop(&mut self) {
        self.node.ref_drop_counter.fetch_add(TaggedPtr::<Node<T, ElemAlloc>>::TAG_UNIT, Ordering::AcqRel);
    }
}
