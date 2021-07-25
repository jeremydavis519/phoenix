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

//! This module defines the [`SysCallFuture`] type, which represents a value that the kernel has
//! promised to give us but may not have provided yet. This type is a core part of the Phoenix
//! system call model, since almost every system call is asynchronous.

use {
    alloc::{
        boxed::Box,
        vec::Vec
    },
    core::{
        future::Future,
        mem,
        pin::Pin,
        ptr,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker}
    },
    crate::syscall
};

/// An executor designed for waiting for the results of asynchronous system calls.
///
/// This type of executor can either poll all of its futures once or block until either one or all
/// of them are complete. Note that blocking with this executor can lead to deadlock if the futures
/// it owns are not waiting for the results of asynchronous system calls.
pub struct SysCallExecutor<'a> {
    futures: Vec<Pin<Box<dyn 'a+Future<Output = ()>>>>
}

impl<'a> SysCallExecutor<'a> {
    /// Makes a new executor with no futures.
    pub fn new() -> Self {
        Self { futures: Vec::new() }
    }

    /// Adds a future to this executor.
    ///
    /// # Returns
    /// `self`, so that multiple function calls can be chained.
    pub fn spawn<F: 'a+Future<Output = ()>>(&mut self, future: F) -> &mut Self {
        self.futures.push(Box::pin(future));
        self
    }

    /// Polls each future once.
    ///
    /// # Returns
    /// The number of futures that finished executing.
    pub fn poll(&mut self) -> usize {
        let mut futures_finished = 0;
        let waker = unsafe { Waker::from_raw(Self::raw_waker()) };
        for i in (0 .. self.futures.len()).rev() {
            let mut cx = Context::from_waker(&waker);
            match self.futures[i].as_mut().poll(&mut cx) {
                Poll::Ready(()) => {
                    futures_finished += 1;
                    self.futures.swap_remove(i);
                },
                Poll::Pending => {}
            };
        }
        futures_finished
    }

    /// Blocks until at least one future finishes.
    ///
    /// # Returns
    /// The number of futures that finished executing.
    pub fn block_on_any(&mut self) -> usize {
        if self.futures.len() == 0 {
            return 0;
        }
        loop {
            let futures_finished = self.poll();
            if futures_finished > 0 {
                return futures_finished;
            }
            syscall::thread_wait();
        }
    }

    /// Blocks until all the futures in this executor finish.
    pub fn block_on_all(&mut self) {
        while self.futures.len() > 0 {
            self.block_on_any();
        }
    }

    fn raw_waker() -> RawWaker {
        RawWaker::new(
            ptr::null(),
            &RawWakerVTable::new(
                |_| Self::raw_waker(), // unsafe fn clone(_: *const ()) -> RawWaker
                |_| {},                // unsafe fn wake(_: *const ())
                |_| {},                // unsafe fn wake_by_ref(_: *const ())
                |_| {}                 // unsafe fn drop(_: *const ())
            )
        )
    }
}

/// A promised but possibly not-yet-available return value from a system call.
pub type SysCallFuture = SysCallFutureInternal<[u8]>;

// This implementation detail is necessary for being able to instantiate `SysCallFuture` as a DST.
#[derive(Debug)]
#[repr(C)]
#[doc(hidden)]
pub struct SysCallFutureInternal<T: ?Sized> {
    finished:  AtomicBool,
    value_len: usize,
    value:     T
}

impl SysCallFuture {
    pub(crate) unsafe fn from_addr<const VALUE_LEN: usize>(addr: usize) -> Pin<&'static mut SysCallFuture> {
        let future = Pin::new_unchecked(
            &mut *(addr as *mut SysCallFutureInternal<[u8; VALUE_LEN]>) as &mut SysCallFuture
        );
        assert_eq!(future.value_len, VALUE_LEN);
        future
    }
}

impl<T: Sized> SysCallFutureInternal<T> {
    /// Marks this future as pending and initializes its size.
    pub fn init_pending(&mut self) {
        self.finished.store(false, Ordering::Release);
        self.value_len = mem::size_of::<T>();
    }

    /// Marks this future as ready, with the given value, and initializes its size.
    pub fn init_ready(&mut self, value: T) {
        self.value = value;
        self.value_len = mem::size_of::<T>();
        self.finished.store(true, Ordering::Release);
    }
}

impl Future for SysCallFuture {
    type Output = Vec<u8>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Vec<u8>> {
        if self.finished.load(Ordering::Acquire) {
            // TODO: This is probably pretty inefficient. Can we optimize it by using a faster allocator?
            Poll::Ready(self.value.to_vec())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl<T: ?Sized> Drop for SysCallFutureInternal<T> {
    fn drop(&mut self) {
        // FIXME: Tell the kernel it can free this future.
    }
}
