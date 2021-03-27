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
    core::{
        mem,
        slice
    },
    bitflags::bitflags,
    libphoenix::{
        phoenix_main,
        future::SysCallExecutor
    },
    libdriver::Device,
    virtio::{DeviceEndian, DeviceDetails, GenericFeatures}
};

// mod mmio;
// mod pci;

phoenix_main! {
    fn main() {
        const DEVICE_TYPE_GPU: u32 = 16;

        let mut device = None;
        SysCallExecutor::new()
            .spawn(async {
                device = Device::claim("mmio/virtio-16").await;
            })
            .block_on_all();
        let device = device.expect("no VirtIO GPU found");
        let _device_details = match virtio::init(
                    &device,
                    DEVICE_TYPE_GPU,
                    mem::size_of::<ConfigurationSpace>()
                ) {
            Ok(x) => x,
            Err(e) => panic!("failed to initialize the VirtIO GPU: {}", e)
        };

        // Event loop
        // TODO: Should this be encapsulated in a library call?
        loop {
            // TODO: Handle events.
            return;
        }
    }
}

struct ConfigurationSpace<'a> {
    regs:   &'a mut [u32],
    legacy: bool
}

#[allow(dead_code)]
impl<'a> ConfigurationSpace<'a> {
    fn new(device_details: &'a mut DeviceDetails) -> Self {
        let legacy = device_details.legacy();
        let byte_slice = device_details.configuration_space();
        let regs = unsafe {
            slice::from_raw_parts_mut(
                byte_slice as *mut _ as *mut u32,
                byte_slice.len() * mem::size_of::<u8>() / mem::size_of::<u32>()
            )
        };
        Self { regs, legacy }
    }

    fn events(&mut self) -> u32 {
        unsafe { u32::from_device_endian((&self.regs[0] as *const u32).read_volatile(), self.legacy) }
    }

    fn clear_events(&mut self, events: u32) -> &Self {
        unsafe { (&mut self.regs[1] as *mut u32).write_volatile(events.to_device_endian(self.legacy)); }
        self
    }

    fn num_scanouts(&mut self) -> u32 {
        unsafe { u32::from_device_endian((&self.regs[2] as *const u32).read_volatile(), self.legacy) }
    }
}

bitflags! {
    struct Features: u64 {
        // GPU-specific
        const GPU_VIRGL = 0x0000_0000_0000_0001;
        const GPU_EDID  = 0x0000_0000_0000_0002;

        // Generic
        const NOTIFY_ON_EMPTY     = GenericFeatures::NOTIFY_ON_EMPTY.bits();
        const ANY_LAYOUT          = GenericFeatures::ANY_LAYOUT.bits();
        const RING_INDIRECT_DESC  = GenericFeatures::RING_INDIRECT_DESC.bits();
        const RING_EVENT_INDEX    = GenericFeatures::RING_EVENT_INDEX.bits();
        const VERSION_1           = GenericFeatures::VERSION_1.bits();
        const ACCESS_PLATFORM     = GenericFeatures::ACCESS_PLATFORM.bits();
        const RING_PACKED         = GenericFeatures::RING_PACKED.bits();
        const IN_ORDER            = GenericFeatures::IN_ORDER.bits();
        const ORDER_PLATFORM      = GenericFeatures::ORDER_PLATFORM.bits();
        const SINGLE_ROOT_IO_VIRT = GenericFeatures::SINGLE_ROOT_IO_VIRT.bits();
        const NOTIFICATION_DATA   = GenericFeatures::NOTIFICATION_DATA.bits();
    }
}
