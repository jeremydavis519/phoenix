/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines all the messages that can be sent between the driver and an HID device.

use {
    core::{
        fmt,
        future::Future,
        mem,
        pin::Pin,
        task::{Context, Poll}
    },
    libphoenix::allocator::PhysBox,
    virtio::{
        virtqueue::{
            SendRecvResult,
            VirtQueue,
        },
        VirtIoError,
    },
};

/// Asynchronously provides a buffer to the device and waits for it to respond with an input event.
pub fn recv_event<'a>(virtq: &'a VirtQueue<'a>, buf: PhysBox<InputEvent>)
        -> impl Future<Output = Result<PhysBox<InputEvent>, InputError>> + 'a {
    let mut result = virtq.send_recv(buf, 0, Some(mem::size_of::<InputEvent>()));

    async move {
        loop {
            match result {
                SendRecvResult::Ok(future) => {
                    let response = future.await;
                    if response.valid_bytes() < mem::size_of::<InputEvent>() {
                        return Err(InputError::InvalidEvent);
                    }
                    return Ok(response.into_buffer());
                },
                SendRecvResult::Retry(buf) => {
                    RelaxFuture::new().await;
                    result = virtq.send_recv(buf, 0, Some(mem::size_of::<InputEvent>()));
                },
                SendRecvResult::Err(e) => return Err(InputError::VirtIo(e)),
            };
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct InputEvent {
    ty:    Le16,
    code:  Le16,
    value: Le32,
}

impl InputEvent {
    pub fn uninit() -> Self {
        Self {
            ty:    u16::max_value().to_le(),
            code:  u16::max_value().to_le(),
            value: u32::max_value().to_le(),
        }
    }

    pub fn ty(&self) -> u16 {
        u16::from_le(self.ty)
    }

    pub fn code(&self) -> u16 {
        u16::from_le(self.code)
    }

    pub fn value(&self) -> u32 {
        u32::from_le(self.value)
    }
}

// These type aliases show when numbers are expected to be in little-endian order. (Newtypes would
// be safer, but also bulkier.)
type Le16 = u16;
type Le32 = u32;

// A future that returns `Pending` once, then `Ready`. The purpose is to allow other futures to run
// while an `async` block waits for an external event.
struct RelaxFuture {
    finished: bool
}

impl RelaxFuture {
    const fn new() -> Self {
        Self { finished: false }
    }
}

impl Future for RelaxFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        if self.finished {
            Poll::Ready(())
        } else {
            self.finished = true;
            Poll::Pending
        }
    }
}

/// An error that might occur while waiting for an input event.
#[derive(Debug)]
pub enum InputError {
    /// An error from the virtio crate.
    VirtIo(VirtIoError),
    /// Indicates that the device provided an invalid event.
    InvalidEvent,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VirtIo(e) => write!(f, "{e}"),
            Self::InvalidEvent => write!(f, "invalid event"),
        }
    }
}
