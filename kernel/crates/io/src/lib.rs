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

//! This crate everything the kernel needs for doing its own input and output with streams of bytes
//! and hardware that operates on streams of bytes, such as serial ports.

#![no_std]

#![feature(allocator_api)]

#![deny(warnings, missing_docs)]

extern crate alloc;
#[cfg(not(target_arch = "x86_64"))]
#[macro_use] extern crate shared;

pub mod serial;
mod std;
pub use std::*;

#[cfg(not(target_arch = "x86_64"))]
use {
    alloc::alloc::AllocError,
    volatile::Volatile,
    i18n::Text,
    memory::{
        allocator::AllMemAlloc,
        phys::block::Mmio
    }
};

// TODO: Move all the GPIO definitions into a proper `gpio` module.
#[cfg(target_machine = "raspi1")] mod gpio {
    pub static GPIO_MMIO_BASE: usize = 0x2020_0000;
    pub static GPIO_MMIO_SIZE: usize = 0x1000; // TODO: Verify this size.
} #[cfg(any(target_machine = "raspi2", target_machine = "raspi3"))] mod gpio {
    pub static GPIO_MMIO_BASE: usize = 0x3f20_0000;
    pub static GPIO_MMIO_SIZE: usize = 0x1000; // TODO: Verify this size.
} #[cfg(target_machine = "qemu-virt")] mod gpio {
    pub static GPIO_MMIO_BASE: usize = 0x0903_0000;
    pub static GPIO_MMIO_SIZE: usize = 0x1000;
}
#[cfg(not(feature = "unit-test"))]
use self::gpio::*;

#[cfg(target_machine = "qemu-virt")]
lazy_static! {
    unsafe {
        // The block of all GPIO registers
        static ref GPIO_MMIO: Mmio<Volatile<u32>> = {
            let result = AllMemAlloc.mmio_mut(GPIO_MMIO_BASE, GPIO_MMIO_SIZE);
            match result {
                Ok(x) => x,
                Err(AllocError) => panic!("{}", Text::GpioCouldntReserveRegs)
            }
        };
    }
}

#[cfg(any(target_machine = "raspi1", target_machine = "raspi2", target_machine = "raspi3"))]
#[derive(Debug, Clone, Copy)]
enum GpioRegs {
    // Controls whether ALL of the GPIO pins can pull their signals up and down.
    GPPUD = 0x94 / 4,
    
    // Controls whether a specific pin can pull its signal up and down.
    GPPUDCLK0 = 0x98 / 4
}
