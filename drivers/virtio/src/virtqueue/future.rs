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

//! This module defines a type that represents a response that the device will place in a virtqueue.

use {
    alloc::{
        boxed::Box,
        sync::Arc,
        vec::Vec
    },
    core::{
        cell::RefCell,
        future::Future,
        mem,
        pin::Pin,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker}
    },
    libphoenix::allocator::PhysBox,
    crate::{DeviceEndian, GenericFeatures},
    super::{Response, VirtQueue}
};

/// An executor that can run most futures without return values, including `async` blocks that run
/// [`VirtQueue::send_recv`]. Specifically, the futures must not rely on being woken up by the
/// executor but must instead use the provided `Waker`s themselves.
pub struct Executor<'a> {
    futures: Vec<Pin<Arc<WakeableFuture<'a>>>>
}

struct WakeableFuture<'a> {
    future: RefCell<Pin<Box<dyn 'a+Future<Output = ()>>>>,
    awake: AtomicBool
}

impl<'a> Executor<'a> {
    /// Makes a new executor with no futures.
    pub fn new() -> Self {
        Self { futures: Vec::new() }
    }

    /// Adds a future to this executor.
    ///
    /// # Returns
    /// `self`, so that multiple function calls can be chained.
    pub fn spawn<F: 'a+Future<Output = ()>>(&mut self, future: F) -> &mut Self {
        self.futures.push(Arc::pin(WakeableFuture {
            future: RefCell::new(Box::pin(future)),
            awake: AtomicBool::new(true)
        }));
        self
    }

    /// Polls each awake future at least once.
    ///
    /// NB: There is no guarantee that a future is polled _only_ once, but this function does
    /// guarantee that the number of polls is bounded and that no future is polled after it has
    /// already finished. There is also no guarantee about the order in which futures are polled.
    ///
    /// # Panics
    /// If there is at least one future and all the futures are asleep.
    ///
    /// # Returns
    /// The number of futures that finished executing.
    pub fn poll(&mut self) -> usize {
        if self.futures.len() == 0 {
            return 0;
        }

        let mut futures_finished = 0;
        let mut some_awake = false;

        for i in (0 .. self.futures.len()).rev() {
            if self.futures[i].awake.swap(false, Ordering::AcqRel) {
                some_awake = true;
                let waker = unsafe { Waker::from_raw(Self::raw_waker(self.futures[i].clone())) };
                let mut cx = Context::from_waker(&waker);
                let mut future = self.futures[i].future.borrow_mut();
                match future.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => {
                        futures_finished += 1;
                        drop(future);
                        self.futures.swap_remove(i);
                    },
                    Poll::Pending => {}
                }
            }
        }

        assert!(some_awake, "all polled futures are asleep");

        futures_finished
    }

    /// Blocks until at least one future finishes.
    ///
    /// # Panics
    /// If, before one of the futures has finished, they are all asleep.
    ///
    /// # Returns
    /// The number of futures that finished executing.
    pub fn block_on_any(&mut self) -> usize {
        loop {
            let futures_finished = self.poll();
            if futures_finished > 0 {
                return futures_finished;
            }
        }
    }

    /// Blocks until all the futures in this executor finish.
    ///
    /// # Panics
    /// If, before all the futures have finished, they are all asleep.
    pub fn block_on_all(&mut self) {
        while self.futures.len() > 0 {
            self.block_on_any();
        }
    }

    fn raw_waker(future: Pin<Arc<WakeableFuture<'a>>>) -> RawWaker {
        let data = unsafe {
            Arc::into_raw(Pin::into_inner_unchecked(future))
        } as *const ();
        RawWaker::new(
            data,
            &RawWakerVTable::new(
                Self::raw_waker_clone,
                Self::raw_waker_wake,
                Self::raw_waker_wake_by_ref,
                Self::raw_waker_drop
            )
        )
    }

    unsafe fn raw_waker_clone(data: *const ()) -> RawWaker {
        let future = Arc::from_raw(data as *const WakeableFuture<'a>);
        let clone = Self::raw_waker(Pin::new(future.clone()));
        drop(Arc::into_raw(future)); // Avoid dropping the Arc.
        clone
    }

    unsafe fn raw_waker_wake(data: *const ()) {
        Self::raw_waker_wake_by_ref(data);
        Self::raw_waker_drop(data);
    }

    unsafe fn raw_waker_wake_by_ref(data: *const ()) {
        let future = Arc::from_raw(data as *const WakeableFuture<'a>);
        future.awake.store(true, Ordering::Release);
        drop(Arc::into_raw(future)); // Avoid dropping the Arc.
    }

    unsafe fn raw_waker_drop(data: *const ()) {
        let future = Arc::from_raw(data as *const WakeableFuture<'a>);
        drop(future);
    }
}

/// A future that evaluates to a [`Response`].
#[derive(Debug)]
pub struct ResponseFuture<'a, T: ?Sized> {
    virtq: Option<&'a VirtQueue<'a>>,
    desc_head_idx: u16,
    desc_tail_idx: u16,
    descriptors_count: u16,
    buffer: Option<PhysBox<T>>,
    legacy_response_len: Option<usize>
}

impl<'a, T: ?Sized> ResponseFuture<'a, T> {
    pub(crate) fn new(
        virtq: &'a VirtQueue<'a>,
        desc_head_idx: u16,
        desc_tail_idx: u16,
        descriptors_count: u16,
        buffer: PhysBox<T>,
        legacy_response_len: Option<usize>
    ) -> Self {
        assert!(descriptors_count > 0);
        Self {
            virtq: Some(virtq),
            desc_head_idx,
            desc_tail_idx,
            descriptors_count,
            buffer: Some(buffer),
            legacy_response_len
        }
    }

    pub(crate) fn new_immediate(buffer: PhysBox<T>) -> Self {
        let legacy_response_len = Some(mem::size_of_val(&*buffer));
        Self {
            virtq: None,
            desc_head_idx: 0,
            desc_tail_idx: 0,
            descriptors_count: 0,
            buffer: Some(buffer),
            legacy_response_len
        }
    }
}

impl<'a, T: ?Sized> Future for ResponseFuture<'a, T> {
    type Output = Response<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match *self {
            ResponseFuture { virtq: None, ref mut buffer, .. } => {
                // This hasn't been tied to a virtqueue, so we can't wait for a response. Just
                // return what's already there. (This is returned by `VirtQueue::send_recv` if
                // we try to send a zero-length message.)
                let buffer = mem::replace(buffer, None)
                    .expect("polled a ResponseFuture that was already finished");
                let valid_bytes = mem::size_of_val(&*buffer);
                Poll::Ready(Response { buffer, valid_bytes })
            },
            ResponseFuture {
                virtq: Some(ref virtq),
                desc_head_idx,
                desc_tail_idx,
                descriptors_count,
                ref mut buffer,
                legacy_response_len
            } => {
                let dev_ring = virtq.device_ring.ring();
                let last_dev_ring_idx = virtq.last_dev_ring_idx.load(Ordering::Acquire);
                let dev_ring_entry = &dev_ring[last_dev_ring_idx as usize % dev_ring.len()];
                let found_desc_idx = u32::from_device_endian(
                    unsafe { (&dev_ring_entry.id as *const u32).read_volatile() },
                    virtq.legacy
                ) as u16;
                if virtq.device_ring.idx() == last_dev_ring_idx {
                    // The device hasn't read any buffers yet. Stay awake so we don't miss it.
                    // PERF: Wait for a "used buffer notification" before waking the appropriate
                    //       future to avoid needless polling.
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else if virtq.device_features & GenericFeatures::IN_ORDER.bits() != 0 {
                    // This device guarantees that it will consume the buffers given to it in order,
                    // and it has just consumed at least one. According to the VirtIO spec
                    // (§ 2.6.9), it's allowed to put a single value in the device ring to indicate
                    // that all the buffers up to and including that one have been used. In order to
                    // handle this correctly, we need to resolve the futures in order as well.

                    let offset = virtq.accumulated_batch_size.load(Ordering::Acquire);
                    let next_idx = last_dev_ring_idx.wrapping_add(offset) % virtq.len();
                    let next_desc_idx = virtq.driver_ring[next_idx].load(Ordering::Acquire);
                    if next_desc_idx == desc_head_idx {
                        // This future's descriptor is next in line. Handle it as above, except we
                        // don't have a `UsedElem` object from the device. That just means we can
                        // assume the device has read or written to every byte in the buffer.

                        virtq.descriptors.dealloc_chain(desc_head_idx, desc_tail_idx, descriptors_count);

                        // We need to keep track of how many descriptor chains are in this batch so
                        // we can skip forward the correct amount.
                        virtq.accumulated_batch_size.fetch_add(1, Ordering::AcqRel);

                        // Wake the next future in line.
                        let next_idx = next_idx.wrapping_add(1) % virtq.len();
                        let next_desc_idx = virtq.driver_ring[next_idx].load(Ordering::Acquire);
                        if let Some(waker) = virtq.wakers[next_desc_idx as usize].replace(None) {
                            waker.wake();
                        }

                        // Return the response.
                        let buffer = mem::replace(buffer, None)
                            .expect("polled a ResponseFuture that was already finished");
                        let valid_bytes = mem::size_of_val(&*buffer);
                        Poll::Ready(Response { buffer, valid_bytes })
                    } else {
                        // This future's descriptor isn't next in line. Wake the correct future and
                        // go to sleep.
                        if let Some(waker) = virtq.wakers[next_desc_idx as usize].replace(None) {
                            waker.wake();
                        }
                        if virtq.wakers[desc_head_idx as usize].borrow().is_some() {
                            panic!("ResponseFuture trying to sleep, but waker slot already taken");
                        }
                        *virtq.wakers[desc_head_idx as usize].borrow_mut() = Some(cx.waker().clone());
                        Poll::Pending
                    }
                } else if found_desc_idx == desc_head_idx {
                    // The device has used this future's buffer.

                    virtq.descriptors.dealloc_chain(desc_head_idx, desc_tail_idx, descriptors_count);

                    // Make sure we look in the right place for the next buffer returned by the
                    // device.
                    let batch_size = virtq.accumulated_batch_size.swap(0, Ordering::AcqRel)
                        .wrapping_add(1);
                    let last_dev_ring_idx = virtq.last_dev_ring_idx.fetch_add(batch_size, Ordering::AcqRel)
                        .wrapping_add(batch_size);

                    // If we haven't gotten through all the available descriptors yet, wake the next
                    // descriptor's future.
                    if last_dev_ring_idx != virtq.driver_ring.idx() {
                        let last_dev_ring_idx = virtq.last_dev_ring_idx.load(Ordering::Acquire);
                        let next_desc_idx = virtq.driver_ring[last_dev_ring_idx % virtq.len()]
                            .load(Ordering::Acquire);
                        if let Some(waker) = virtq.wakers[next_desc_idx as usize].replace(None) {
                            waker.wake();
                        }
                    }

                    // Return the response.
                    let buffer = mem::replace(buffer, None)
                        .expect("polled a ResponseFuture that was already finished");
                    let valid_bytes = if virtq.legacy {
                        match legacy_response_len {
                            Some(legacy_response_len) => legacy_response_len,
                            None => u32::from_device_endian(
                                unsafe { (&dev_ring_entry.len as *const u32).read_volatile() },
                                virtq.legacy
                            ) as usize
                        }
                    } else {
                        u32::from_device_endian(
                            unsafe { (&dev_ring_entry.len as *const u32).read_volatile() },
                            virtq.legacy
                        ) as usize
                    };
                    Poll::Ready(Response { buffer, valid_bytes })
                } else {
                    // The device has read at least one buffer, but it's not one that concerns
                    // this future. Wake the future that's responsible for this descriptor chain
                    // and wait until someone wakes us up.

                    if let Some(waker) = virtq.wakers[found_desc_idx as usize].replace(None) {
                        waker.wake();
                    }
                    if virtq.wakers[desc_head_idx as usize].borrow().is_some() {
                        panic!("ResponseFuture trying to sleep, but waker slot already taken");
                    }
                    *virtq.wakers[desc_head_idx as usize].borrow_mut() = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}
