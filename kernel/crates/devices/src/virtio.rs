/* Copyright (c) 2019-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the interface for the kernel to enumerate Virtual I/O (VirtIO) devices.

use {
    alloc::{
        vec::Vec,
        string::String,
    },
    core::{
        fmt::Write,
        mem,
        sync::atomic::AtomicBool
    },
    libdriver::BusType,
    shared::{
        ffi_enum,
        ffi::{Endian, Le}
    },
    crate::{
        DeviceTree,
        bus::Bus
    }
};

const MMIO_MAGIC_NUMBER: u32 = 0x74726976; // Little-endian "virt"

/// Inserts all the VirtIO devices into the given device tree.
pub fn enumerate(device_tree: &mut DeviceTree) -> Result<(), ()> {
    match *device_tree {
        DeviceTree::Root { ref mut children } => {
            // Look for VirtIO devices on every bus.
            for child in children {
                enumerate(child)?;
            }
        },
        DeviceTree::Mmio { ref bus, ref mut children } => {
            #[cfg(target_machine = "qemu-virt")] mod temp {
                pub const MMIO_BASE: usize            = 0x0a00_0000;
                pub const MMIO_SIZE_PER_DEVICE: usize = 0x0200;
                pub const MMIO_MAX_DEVICES: usize     = 32;
            }
            #[cfg(target_arch = "x86_64")] mod temp {
                // FIXME: Is there a generic way to find MMIO VirtIO devices on x86-64? Or should we
                // assume they'll always be on PCI buses?
                pub const MMIO_BASE: usize            = 0;
                pub const MMIO_SIZE_PER_DEVICE: usize = 0;
                pub const MMIO_MAX_DEVICES: usize     = 0;
            }
            use temp::*;

            for i in 0 .. MMIO_MAX_DEVICES {
                let resource = bus.reserve(MMIO_BASE + i * MMIO_SIZE_PER_DEVICE, MMIO_SIZE_PER_DEVICE)
                    .map_err(|_| ())?;
                match resource.bus {
                    BusType::Mmio => {
                        unsafe {
                            if (*(resource.base as *const Le<u32>)).into_native() == MMIO_MAGIC_NUMBER {
                                let device_id = (*(resource.base.wrapping_add(8) as *const Le<u32>)).into_native();
                                if device_id != 0 {
                                    // It seems we've found a VirtIO device. Add it to the tree.
                                    let mut name = String::new();
                                    name.try_reserve(mem::size_of_val("virtio-4294967295"))
                                        .map_err(|_| ())?;
                                    write!(name, "virtio-{}", device_id).unwrap();

                                    let mut resources = Vec::new();
                                    resources.try_reserve(1)
                                        .map_err(|_| ())?;
                                    resources.push(resource);

                                    children.try_reserve(1)
                                        .map_err(|_| ())?;
                                    children.push(DeviceTree::Device {
                                        name,
                                        claimed: AtomicBool::new(false),
                                        resources
                                    });
                                }
                            }
                        }
                    },
                    // bus => panic!("unexpected bus type for VirtIO resource: {:?}", bus),
                };
            }
        },
        DeviceTree::Device { .. } => {} // There can't be a VirtIO device inside another device.
    };
    Ok(())
}

ffi_enum! {
    #[repr(u32)]
    #[derive(Debug, Clone, Copy)]
    /// Represents the type of a VirtIO device.
    // TODO: Make a driver for each of these, or at least those that are defined in the VirtIO
    // specification.
    pub enum DeviceType {
        /// A network card
        NetworkCard           =  1,
        /// A block device such as a hard drive
        BlockDevice           =  2,
        /// A text console
        Console               =  3,
        /// A source of entropy for random number generation
        EntropySource         =  4,
        /// A device for allowing the host to change the amount of memory the guest has (old
        /// interface)
        MemBalloonTraditional =  5,
        /// ???
        IoMemory              =  6,
        /// ???
        Rpmsg                 =  7,
        /// An SCSI host that handles one or more "virtual logical units (such as disks)"
        ScsiHost              =  8,
        /// ???
        Transport9P           =  9,
        /// ??? A wireless network card?
        Mac80211Wlan          = 10,
        /// ??? A serial port?
        RprocSerial           = 11,
        /// ???
        VirtIoCaif            = 12,
        /// A device for allowing the host to change the amount of memory the guest has (new
        /// interface)
        MemBalloon            = 13,
        /// A video card
        Gpu                   = 16,
        /// A timer/clock
        Timer                 = 17,
        /// An input device, such as a keyboard, mouse, or touchscreen
        Input                 = 18,
        /// A socket device for communication between the host and guest
        Socket                = 19,
        /// A cryptographic accelerator
        Crypto                = 20,
        /// ???
        SignalDistModule      = 21,
        /// ???
        Pstore                = 22,
        /// ???
        IoMmu                 = 23,
        /// ??? Expanded memory?
        Memory                = 24
    }
}
