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

extern crate alloc;

use {
    alloc::{
        string::String,
        vec::Vec
    },
    core::{
        mem,
        num::NonZeroUsize,
        slice,
        sync::atomic::{AtomicBool, Ordering}
    },
    libdriver::{BusType, DeviceContents, Resource},
    memory::{
        allocator::AllMemAlloc,
        phys::RegionType,
        virt::paging::{self, RootPageTable}
    },
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
            Err(()) => panic!("failed to construct the device tree") // FIXME: Show the reason for failure.
        };
    }
}

fn devices() -> Result<DeviceTree, ()> {
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
        name:      String,
        /// Whether or not a driver is currently asserting ownership of this device.
        claimed:   AtomicBool,
        /// The resources (MMIO, ISA ports, etc.) that the device owns.
        resources: Vec<Resource>
    }
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
    /// * `Ok(addr)` if the device exists, the process has permission to claim it, it has not
    ///   already been claimed, and we successfully gave the process access to the device's
    ///   resources. `addr` is the userspace address of the constructed `DeviceContents` object.
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
                ref resources
            } => {
                if let Some(name_tail) = path.match_and_advance(name) {
                    if !name_tail.is_empty() {
                        // The device name is only a prefix of the requested name.
                        return Err(());
                    }
                    // FIXME: Fail here if the process doesn't have permission to claim this device.
                    if claimed.swap(true, Ordering::AcqRel) {
                        // Another driver has already claimed this device.
                        return Err(());
                    }

                    let page_size = paging::page_size();

                    // Construct the `DeviceContents` object that will tell the userspace process
                    // how to access this device's resources.
                    // TODO: We could save some memory by putting multiple `DeviceContents` objects
                    //       in one page.
                    let device_contents_size = DeviceContents::size_with_resources(resources.len());
                    let device_contents_block = AllMemAlloc.malloc::<u8>(
                        device_contents_size.wrapping_add(page_size - 1) / page_size * page_size,
                        NonZeroUsize::new(usize::max(mem::align_of::<DeviceContents>(), page_size)).unwrap()
                    )
                        .map_err(|_| ())?;

                    let device_contents = unsafe {
                        &mut *(device_contents_block.index(0) as *mut DeviceContents)
                    };
                    let device_contents_resources = unsafe {
                        slice::from_raw_parts_mut(
                            &mut device_contents.resources as *mut [Resource; 0] as *mut Resource,
                            resources.len()
                        )
                    };

                    mem::forget(mem::replace(&mut device_contents.resources_count, resources.len()));

                    // Give the process access to the device's resources.
                    for (i, resource) in resources.iter().enumerate() {
                        match resource.bus {
                            BusType::Mmio => {
                                // FIXME: If the resource is not page-aligned and page-sized and
                                //        the device doesn't have a certain permission
                                //        ("unsafe direct unaligned mmio"?), map it into the
                                //        process's address space in a new way. The ONE bit should
                                //        be unset so any access will cause a fault, and the other
                                //        bits should indicate to the kernel that the page grants
                                //        access to this resource. (Any pages in the middle,
                                //        however, can be mapped normally.) The kernel can then
                                //        perform any attempted access on behalf of the process
                                //        after checking that it is within the bounds of the
                                //        resource. (Note that multiple resources might use the same
                                //        page.)
                                //
                                //        The current implementation allows a process to access
                                //        registers of devices that are mapped near the one to which
                                //        it has requested access without requesting access to them.

                                let end_phys = resource.base.wrapping_add(resource.size).wrapping_add(page_size - 1)
                                    / page_size * page_size;
                                let base_phys = resource.base / page_size * page_size;
                                if let Some(size) = NonZeroUsize::new(end_phys.wrapping_sub(base_phys)) {
                                    let userspace_addr = root_page_table.map(
                                        base_phys,
                                        None,
                                        size,
                                        RegionType::Mmio
                                    )? + resource.base % page_size;
                                    mem::forget(mem::replace(&mut device_contents_resources[i], Resource {
                                        bus: resource.bus,
                                        base: userspace_addr,
                                        size: resource.size
                                    }));
                                }
                            }
                        };
                    }

                    // Map the `DeviceContents` into the process's address space and tell the
                    // process where to find it.
                    let userspace_addr = root_page_table.map(
                        device_contents_block.base().as_addr_phys(),
                        None,
                        NonZeroUsize::new(device_contents_block.size()).unwrap(),
                        RegionType::Rom
                    )?;
                    // FIXME: Instead of forgetting the block, transfer ownership of it to the process,
                    // so that it will be freed when the process is terminated.
                    mem::forget(device_contents_block);
                    return Ok(userspace_addr);
                }
                Err(())
            }
        }
    }
}
