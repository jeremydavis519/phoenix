/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module provides a platform-independent API for discovering and enumerating hardware
//! devices.

#![no_std]

#![deny(warnings, missing_docs)]

#![feature(try_reserve)]

extern crate alloc;

use {
    alloc::{
        collections::TryReserveError,
        vec::Vec
    },
    shared::lazy_static
};

pub mod bus;
pub mod virtio;

lazy_static! {
    unsafe {
        /// The device tree.
        pub static ref DEVICES: DeviceTree = match devices() {
            Ok(x) => x,
            Err(e) => panic!("failed to construct the device tree: {}", e)
        };
    }
}

fn devices() -> Result<DeviceTree, TryReserveError> {
    let mut devices = DeviceTree::new();

    bus::enumerate(&mut devices)?;
    virtio::enumerate(&mut devices)?;

    Ok(devices)
}

/// Represents a device tree. The branch nodes are buses, and the leaves are devices located on
/// those buses.
#[derive(Debug)]
pub enum DeviceTree {
    /// The whole system. There should only ever be one root.
    Root {
        /// The devices and/or buses found on this bus.
        children: Vec<DeviceTree>
    },
    /// A piece of the memory space viewed as an I/O bus.
    Mmio {
        /// The bus itself.
        bus: bus::mmio::MmioBus,
        /// The devices and/or buses found on this bus.
        children: Vec<DeviceTree>
    },
    // TODO:
    // /// A PCI bus.
    // Pci {
    //     /// The bus itself.
    //     bus: bus::pci::Bus,
    //     /// The devices and/or buses found on this bus.
    //     children: Vec<DeviceTree>
    // },

    /// A device.
    Device(DeviceEnum)
}

/// Represents a device of any type we support.
#[derive(Debug)]
pub enum DeviceEnum {
    /// A VirtIO device.
    VirtIo(virtio::Device)
}

impl DeviceTree {
    fn new() -> DeviceTree {
        DeviceTree::Root { children: Vec::new() }
    }
}
