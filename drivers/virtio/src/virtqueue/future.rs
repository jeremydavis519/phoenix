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
    alloc::vec::Vec,
    core::{
        sync::atomic::Ordering,
        future::Future,
        mem,
        pin::Pin,
        task::{Context, Poll}
    },
    libphoenix::allocator::PhysBox,
    crate::{DeviceEndian, GenericFeatures},
    super::VirtQueue
};

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
    pub fn new(
        virtq: &'a VirtQueue<'a>,
        desc_head_idx: u16,
        desc_tail_idx: u16,
        descriptors_count: u16,
        buffer: PhysBox<T>,
        legacy_response_len: Option<usize>
    ) -> Self {
        Self {
            virtq: Some(virtq),
            desc_head_idx,
            desc_tail_idx,
            descriptors_count,
            buffer: Some(buffer),
            legacy_response_len
        }
    }

    pub fn new_immediate(buffer: PhysBox<T>) -> Self {
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

#[derive(Debug)]
pub struct Response<T: ?Sized> {
    buffer: PhysBox<T>,
    valid_bytes: usize // The number of bytes from the beginning of `*buffer` that are defined
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
                // Returns the descriptor chain to the list of free descriptors so they can be used
                // by other futures later.
                let dealloc_descriptor_chain = || {
                    assert!(descriptors_count > 0);
                    let mut next = virtq.descriptors.first_free_idx.load(Ordering::Acquire); // Device-endian
                    loop {
                        let desc_tail = &virtq.descriptors[desc_tail_idx as usize];
                        desc_tail.next.store(next, Ordering::Release);
                        match virtq.descriptors.first_free_idx.compare_exchange_weak(
                                next,
                                desc_head_idx,
                                Ordering::AcqRel,
                                Ordering::Acquire
                        ) {
                            Ok(_) => break,
                            Err(x) => next = x // The list has a new head. Retry with that one.
                        }
                    }
                    virtq.descriptors.free_descs.fetch_add(descriptors_count, Ordering::AcqRel);
                };

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
                } else if found_desc_idx == desc_head_idx {
                    // The device has used this future's buffer.

                    dealloc_descriptor_chain();

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
                        let next_desc_idx = virtq.driver_ring[last_dev_ring_idx as usize % virtq.len()]
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
                } else if virtq.device_features & GenericFeatures::IN_ORDER.bits() != 0 {
                    // This device guarantees that it will consume the buffers given to it in order,
                    // and it has just consumed at least one. According to the VirtIO spec
                    // (§ 2.6.9), it's allowed to put a single value in the device ring to indicate
                    // that all the buffers up to and including that one have been used. In order to
                    // handle this correctly, we need to resolve the futures in order as well.

                    let offset = virtq.accumulated_batch_size.load(Ordering::Acquire);
                    let next_idx = last_dev_ring_idx.wrapping_add(offset) as usize % virtq.len();
                    let next_desc_idx = virtq.driver_ring[next_idx].load(Ordering::Acquire);
                    if next_desc_idx == desc_head_idx {
                        // This future's descriptor is next in line. Handle it as above, except we
                        // don't have a `UsedElem` object from the device. That just means we can
                        // assume the device has read or written to every byte in the buffer.

                        dealloc_descriptor_chain();

                        // We need to keep track of how many descriptor chains are in this batch so
                        // we can skip forward the correct amount.
                        virtq.accumulated_batch_size.fetch_add(1, Ordering::AcqRel);

                        // Wake the next future in line.
                        let next_idx = next_idx.wrapping_add(1) as usize % virtq.len();
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
                        if let None = *virtq.wakers[desc_head_idx as usize].borrow() {
                            panic!("ResponseFuture trying to sleep, but waker slot already taken");
                        }
                        *virtq.wakers[desc_head_idx as usize].borrow_mut() = Some(cx.waker().clone());
                        Poll::Pending
                    }
                } else {
                    // The device has read at least one buffer, but it's not one that concerns
                    // this future. Wake the future that's responsible for this descriptor chain
                    // and wait until someone wakes us up.

                    if let Some(waker) = virtq.wakers[found_desc_idx as usize].replace(None) {
                        waker.wake();
                    }
                    if let None = *virtq.wakers[desc_head_idx as usize].borrow() {
                        panic!("ResponseFuture trying to sleep, but waker slot already taken");
                    }
                    *virtq.wakers[desc_head_idx as usize].borrow_mut() = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}
