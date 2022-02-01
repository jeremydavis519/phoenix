/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines how the kernel deals with IRQs, including interfacing with interrupt
//! controllers and registering and routing interrupt service routines. It does not deal with
//! CPU exceptions and software-generated interrupts. See the `int` crate for those.

#![no_std]

//#![feature(auto_traits)]
//#![feature(allocator_api)]
//#![feature(core_intrinsics)]

#![deny(warnings, missing_docs)]

// TODO: Can we unit-test this module at all?
#![cfg(not(feature = "unit-test"))]

#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
mod arm;
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
use self::arm as self_impl;

pub use self::self_impl::interrupt_controller::irq::{
    register_irq,
    IsrPtr,
    Priority,
    IrqTrigger
};

/// Any function that can be used as an ISR.
pub type IsrFn = fn() -> IsrResult;

/// The required return value of an ISR. It exists in order to allow multiple devices to share the
/// same IRQ if necessary, having only to deal with slower response times from the CPU instead of
/// being completely unable to function.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub enum IsrResult {
    /// The IRQ has been successfully serviced.
    Serviced,
    /// The IRQ wasn't serviced because this was the wrong ISR.
    WrongIsr,
    /// The IRQ was successfully serviced, and the current thread should be pre-empted.
    PreemptThread
}
