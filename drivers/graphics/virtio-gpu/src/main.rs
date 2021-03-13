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

//! This program is the Phoenix operating system's driver for the VirtIO GPU.
//!
//! # Required permissions:
//! * own device */virtio/gpu

#![no_std]
#![deny(warnings, missing_docs)]

#![feature(asm)]

// use libdriver::IoType;
use libphoenix::block_on;

// mod mmio;
// mod pci;

fn main() -> Result<(), ()> {
    let device = libphoenix::block_on(
        libdriver::get_device("*/virtio/gpu")
    )
        .map_err(|e| { eprintln!("Driver initialization failed: {}", e); })?;
    /*match device.io_type {
        IoType::Mmio => self::mmio::init(&device),
        IoType::Pci => self::pci::init(&device)
    };*/

    // Event loop
    // TODO: Should this be encapsulated in a library call?
    loop {
        // TODO: Handle events.
        return Ok(());
    }
}
