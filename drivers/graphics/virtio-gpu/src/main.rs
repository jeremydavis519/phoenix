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
//! * own device mmio/virtio-16

#![no_std]
#![deny(warnings, missing_docs)]

#![feature(default_alloc_error_handler)]
#![feature(start)]

use {
    // libdriver::IoType,
    libphoenix::{
        phoenix_main,
        future::SysCallExecutor
    }
};

// mod mmio;
// mod pci;

phoenix_main! {
    fn main() {
        let mut device = None;
        SysCallExecutor::new()
            .spawn(async {
                device = libdriver::Device::claim("mmio/virtio-16").await
            })
            .block_on_all();
        let _device = device.expect("no VirtIO GPU found");
        /*match device.io_type {
            IoType::Mmio => self::mmio::init(&device),
            IoType::Pci => self::pci::init(&device),
            t => panic!("unexpected I/O type {:?}", t)
        };*/

        // Event loop
        // TODO: Should this be encapsulated in a library call?
        loop {
            // TODO: Handle events.
            return;
        }
    }
}
