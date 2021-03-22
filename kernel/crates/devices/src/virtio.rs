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
        collections::TryReserveError,
        vec::Vec,
        string::String,
    },
    core::{
        fmt::Write,
        mem,
        sync::atomic::AtomicBool
    },
    shared::{
        ffi_enum,
        ffi::{Endian, Le}
    },
    crate::{
        DeviceTree,
        bus::Bus,
        resource::Resource
    }
};

const MMIO_MAGIC_NUMBER: u32 = 0x74726976; // Little-endian "virt"

/// Inserts all the VirtIO devices into the given device tree.
pub fn enumerate(device_tree: &mut DeviceTree) -> Result<(), TryReserveError> {
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
                match bus.reserve(MMIO_BASE + i * MMIO_SIZE_PER_DEVICE, MMIO_SIZE_PER_DEVICE) {
                    Ok(Resource::Mmio(registers)) => {
                        unsafe {
                            if (*(registers.index(0) as *const Le<u32>)).into_native() == MMIO_MAGIC_NUMBER {
                                let device_id = (*(registers.index(8) as *const Le<u32>)).into_native();
                                if device_id != 0 {
                                    // It seems we've found a VirtIO device. Add it to the tree.
                                    let mut name = String::new();
                                    name.try_reserve(mem::size_of_val("virtio-4294967295"))?;
                                    write!(name, "virtio-{}", device_id).unwrap();

                                    let mut resources = Vec::new();
                                    resources.try_reserve(1)?;
                                    resources.push(Resource::Mmio(registers));

                                    children.try_reserve(1)?;
                                    children.push(DeviceTree::Device {
                                        name,
                                        claimed: AtomicBool::new(false),
                                        resources
                                    });
                                }
                            }
                        }
                    },
                    // Ok(resource) => panic!("unexpected resource type: {:?}", resource),
                    Err(_) => {}
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

// FIXME: Move all this stuff into the userspace driver.
/*const CURRENT_VERSION: usize   = 2;

impl Device {
    /// Reads the device's magic number (should be 0x74726976).
    pub unsafe fn magic_number(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x000)
        }.into_native()
    }

    /// Reads the VirtIO version number from the device.
    pub unsafe fn version(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x004)
        }.into_native()
    }

    /// Reads the device ID.
    pub unsafe fn device_id(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x008)
        }.into_native()
    }

    /// Reads the vendor ID.
    pub unsafe fn vendor_id(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x00c)
        }.into_native()
    }

    /// Reads the features that the device supports.
    pub unsafe fn device_features(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x010)
        }.into_native()
    }

    /// Sets the mask for reading the device features.
    pub unsafe fn mask_device_features(&mut self, mask: u32) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x014, Le::from_native(mask))
        }
    }

    /// Sets the features that the driver supports.
    pub unsafe fn set_driver_features(&mut self, features: u32) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x020, Le::from_native(features))
        }
    }

    /// Sets the mask for writing the driver features.
    pub unsafe fn mask_driver_features(&mut self, mask: u32) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x024, Le::from_native(mask))
        }
    }

    /// Selects a virtqueue.
    pub unsafe fn select_queue(&mut self, queue: u32) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x030, Le::from_native(queue))
        }
    }

    /// Reads the maximum length of the selected queue that the device supports.
    pub unsafe fn max_queue_len(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x034)
        }.into_native()
    }

    /// Tells the device the actual size of the selected queue.
    pub unsafe fn set_queue_len(&mut self, len: u32) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x038, Le::from_native(len))
        }
    }

    /// Tells the device whether it can start using the selected queue.
    pub unsafe fn set_queue_ready(&mut self, ready: bool) {
        let raw: u32 = if ready { 1 } else { 0 };
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x044, Le::from_native(raw))
        }
    }

    /// Reads the last value written via `set_queue_ready`.
    pub unsafe fn queue_ready(&self) -> bool {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x044)
        }.into_native() != 0
    }

    /// Notifies the device that there are new buffers in a queue.
    pub unsafe fn notify_queue(&mut self, queue: u32) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x050, Le::from_native(queue))
        }
    }

    /// Reads the reasons why the device raised an interrupt.
    pub unsafe fn interrupt_status(&self) -> InterruptStatus {
        InterruptStatus::from_bits_truncate(match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read(0x060)
        })
    }

    /// Notifies the device that the interrupts indicated by the given bits have been handled.
    pub unsafe fn ack_interrupt(&mut self, ints: InterruptStatus) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x064, ints.bits())
        }
    }

    /// Sets the device status flags, indicating progress in setting up the driver. Writing 0
    /// resets the device.
    pub unsafe fn set_device_status(&mut self, status: DeviceStatus) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.write(0x070, status.bits())
        }
    }

    /// Reads the device status flags.
    pub unsafe fn device_status(&self) -> DeviceStatus {
        DeviceStatus::from_bits_truncate(match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read(0x070)
        })
    }

    /// Resets the device.
    pub unsafe fn reset_device(&mut self) { self.set_device_status(DeviceStatus::empty()) }

    /// Tells the device the physical address of the Descriptor Area of the selected queue.
    pub unsafe fn set_queue_desc_table_addr(&mut self, addr: u64) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => {
                registers.write(0x080, Le::from_native((addr & 0xffff_ffff) as u32));
                registers.write(0x084, Le::from_native((addr >> 32) as u32))
            }
        }
    }

    /// Tells the device the physical address of the Driver Area of the selected queue.
    pub unsafe fn set_driver_ring_addr(&mut self, addr: u64) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => {
                registers.write(0x090, Le::from_native((addr & 0xffff_ffff) as u32));
                registers.write(0x094, Le::from_native((addr >> 32) as u32))
            }
        }
    }

    /// Tells the device the physical address of the Device Area of the selected queue.
    pub unsafe fn set_device_ring_addr(&mut self, addr: u64) {
        match self.resources {
            VirtioResources::Mmio { ref registers } => {
                registers.write(0x0a0, Le::from_native((addr & 0xffff_ffff) as u32));
                registers.write(0x0a4, Le::from_native((addr >> 32) as u32))
            }
        }
    }

    /// Reads the configuration space's generation number. When reading the configuration space, if
    /// this number changes, that means something changed and we need to start over.
    pub unsafe fn config_generation(&self) -> u32 {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read::<Le<u32>>(0x0fc)
        }.into_native()
    }

    /// Reads a value from the device-specific configuration space. Offsets are measured in bytes.
    pub unsafe fn read_config_space<T: Copy>(&self, offset: usize) -> T {
        match self.resources {
            VirtioResources::Mmio { ref registers } => registers.read(0x100 + offset)
        }
    }
}

bitflags! {
    /// Indicates a reason for the device to have raised an interrupt. These values are also used
    /// by the driver to acknowledge the interrupts after handling them.
    pub struct InterruptStatus: u32 {
        /// The device used a buffer.
        const USED_BUFFER    = 0x1_u32.to_le();
        /// The device's configuration has changed.
        const CONFIG_CHANGED = 0x2_u32.to_le();
    }
}

bitflags! {
    /// The current status of the device, used to track the driver's progress in initialization and
    /// any problems that it encounters.
    pub struct DeviceStatus: u32 {
        /// The guest OS has found the device and recognized it as a valid VirtIO device.
        const ACKNOWLEDGED = 0x01_u32.to_le();
        /// The guest OS knows how to drive the device.
        const DRIVER_FOUND = 0x02_u32.to_le();
        /// The driver is set up and ready to drive the device.
        const DRIVER_OK    = 0x04_u32.to_le();
        /// Feature negotiation is complete.
        const FEATURES_OK  = 0x08_u32.to_le();
        /// The device has encountered an unrecoverable error.
        const NEEDS_RESET  = 0x40_u32.to_le();
        /// The guest OS has given up on the device.
        const FAILED       = 0x80_u32.to_le();
    }
}*/
