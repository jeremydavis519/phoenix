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
//! devices. Note that a single physical device might be represented by more than one entry in the
//! device tree. For instance, a VGA-compatible video card will have one entry on the MMIO bus
//! representing its memory-mapped framebuffer and another entry on the ISA bus representing its
//! assigned ISA ports.

#![no_std]

#![deny(warnings, missing_docs)]

#![feature(try_reserve)]

extern crate alloc;

use {
    alloc::{
        collections::TryReserveError,
        string::String,
        vec::Vec
    },
    core::sync::atomic::{AtomicBool, Ordering},
    memory::virt::paging::RootPageTable,
    shared::lazy_static,
    userspace::UserspaceStr
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
    //FIXME: This should be done as part of enumerating the devices on buses, not afterward.
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
    // TODO:
    // /// The x86 ISA bus.
    // Isa {
    //     /// The bus itself.
    //     bus: bus::isa::Bus,
    //     /// The devices and/or buses found on this bus.
    //     children: Vec<DeviceTree>
    // },

    /// A device.
    Device {
        /// The name of the device, which a driver can use to refer to it.
        name:    String,
        /// Whether or not a driver is currently asserting ownership of this device.
        claimed: AtomicBool,
        /// The device itself.
        // FIXME: We should be able to simplify this to just a vector of resource specifications.
        device:  DeviceEnum
    }
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

    /// Marks the named device as claimed and retrieves its information.
    ///
    /// The device path should be structured like a file path, with levels of the device tree
    /// separated by slashes.
    ///
    /// This function does several things as part of claiming the device:
    /// * The process's permissions are checked to see if it is allowed to claim this device.
    /// * The device is atomically marked as claimed so no other driver can access it.
    /// * A [`DeviceContents`] object is constructed on the heap and filled with information the
    ///   driver might need.
    /// * The device's resources are made available to the process. (For instance, MMIO resources
    ///   are mapped to its address space.)
    /// * The `DeviceContents` object is mapped to one or more consecutive free pages in the
    ///   process's address space.
    ///
    /// # Returns
    /// * `Ok(addr)` if the device exists, and the process has permission to claim it, and it has
    ///   not already been claimed. `addr` is the userspace address of the constructed
    ///   `DeviceContents` object.
    /// * `Err(())` otherwise.
    pub fn claim_device(&self, path: UserspaceStr, root_page_table: &RootPageTable) -> Result<usize, ()> {
        // FIXME: Check the process's permissions during this tree walk. (We need to do it carefully,
        //        since we're not copying the string from userspace into the kernel. It needs to be
        //        done one byte at a time, at the same time as we're comparing a byte of the given
        //        path to a device's path.)
        match *self {
            DeviceTree::Root { ref children } => {
                for child in children {
                    if let Ok(addr) = child.claim_device(path.clone(), root_page_table) {
                        return Ok(addr);
                    }
                }
                Err(())
            },
            DeviceTree::Mmio { ref children, .. } => {
                if let Some(path) = path.match_and_advance("mmio/") {
                    for child in children {
                        if let Ok(addr) = child.claim_device(path.clone(), root_page_table) {
                            return Ok(addr);
                        }
                    }
                }
                Err(())
            },
            DeviceTree::Device {
                ref name,
                ref claimed,
                device: _
            } => {
                if let Some(name_tail) = path.match_and_advance(name) {
                    if !name_tail.is_empty() {
                        // The device name is only a prefix of the requested name.
                        return Err(());
                    }
                    if claimed.swap(true, Ordering::AcqRel) {
                        // Another driver has already claimed this device.
                        return Err(());
                    }
                    panic!("Successfully claimed device `{}`! Now finish the job.", name);
                    // TODO
                }
                Err(())
            }
        }
    }
}
