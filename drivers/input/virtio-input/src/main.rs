/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This program is the Phoenix operating system's driver for VirtIO input devices ([specification]).
//! [specification]: https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-3390008
//!
//! # Required permissions:
//! * own device mmio/virtio-18

#![no_std]
#![deny(/*warnings, */missing_docs)]

#![feature(allocator_api)]
#![feature(default_alloc_error_handler)]
#![feature(inline_const)]
#![feature(start)]

extern crate alloc;

use {
    alloc::{
        vec::Vec,
        string::String,
    },
    core::{
        arch::asm,
        fmt::{self, Write},
        future::Future,
        iter,
        mem,
        slice,
    },
    volatile::Volatile,
    libphoenix::{
        allocator::{Allocator, PhysBox},
        syscall,
    },
    libdriver::Device,
    virtio::{
        DeviceEndian, DeviceDetails, GenericFeatures,
        virtqueue::{
            VirtQueue,
            future::Executor,
        }
    },
    //self::api::*,
    self::msg::InputEvent,
};

//mod api;
mod msg;

const DEVICE_TYPE_INPUT: u32 = 18;

fn main() {
    let devices = iter::from_fn(|| Device::claim("mmio/virtio-18"))
        .collect::<Vec<_>>();

    let mut virtio_devices = devices.iter()
        .filter_map(|device| {
            let mut details = match virtio::init(
                device,
                DEVICE_TYPE_INPUT,
                ConfigurationSpace::SIZE,
                QueueIndex::Count as u32,
                GenericFeatures::empty().bits(),
                (
                    GenericFeatures::ANY_LAYOUT |
                    GenericFeatures::VERSION_1 |
                    GenericFeatures::IN_ORDER |
                    GenericFeatures::ORDER_PLATFORM
                ).bits(),
            ) {
                Ok(x) => x,
                Err(e) => {
                    let _ = write!(KernelWriter, "virtio-input: failed to initialize a device: {e}");
                    return None;
                },
            };

            let virtqueues = details.virtqueues();

            Some(VirtIoDevice {
                details,
                virtqueues,
            })
        })
        .collect::<Vec<_>>();

    let mut executor = Executor::new();

    for device in virtio_devices.iter_mut() {
        // Log device information.
        let mut config_space = ConfigurationSpace::new(&mut device.details);

        let _ = writeln!(KernelWriter, "virtio-input: found a device");

        let mut name = ConfigurationValue::uninit_string();
        let size = config_space.read(ConfigSelect::IdName as u8, 0, &mut name);
        let name = unsafe { slice::from_raw_parts(&name.string[0], size.into()) };
        let _ = writeln!(KernelWriter, "virtio-input: name = `{}`", String::from_utf8_lossy(name));

        let mut serial_number = ConfigurationValue::uninit_string();
        let size = config_space.read(ConfigSelect::IdSerial as u8, 0, &mut serial_number);
        let serial_number = unsafe { slice::from_raw_parts(&serial_number.string[0], size.into()) };
        let _ = writeln!(KernelWriter, "virtio-input: serial number = `{}`", String::from_utf8_lossy(serial_number));

        let mut dev_ids = ConfigurationValue { ids: DevIds::default() };
        let _ = config_space.read(ConfigSelect::IdDevIds as u8, 0, &mut dev_ids);
        let dev_ids = unsafe { &dev_ids.ids };
        let _ = writeln!(KernelWriter, "virtio-input: device IDs: {dev_ids}");

        let mut properties = ConfigurationValue::uninit_bitmap();
        let size = config_space.read(ConfigSelect::PropBits as u8, 0, &mut properties);
        let properties = unsafe { slice::from_raw_parts(&properties.bitmap[0], size.into()) };
        let _ = writeln!(KernelWriter, "virtio-input: input properties: {:?}", properties);

        let _ = writeln!(KernelWriter, "virtio-input: VirtIO features: {:#x}", device.details.features());

        // Start listening for events.
        // TODO: Instead of making a new future for every buffer, add a new API to the virtio crate
        //       to add a lot of buffers all at once and respond to each of them with a callback. The
        //       existing API works well when sending individual commands but is hard to use when
        //       simply providing a lot of buffers to the device for input.
        const INITIAL_BUFFERS_COUNT: usize = 16; // QEMU expects more than 1.
        for _ in 0 .. INITIAL_BUFFERS_COUNT {
            executor.spawn(new_event_future(&device.virtqueues[QueueIndex::Event as usize]));
        }
    }

    executor.block_on_all();
}

fn new_event_future<'a>(event_q: &'a VirtQueue<'a>) -> impl Future<Output = ()> + 'a {
    // PERF: Allocate all these buffers in one block.
    const MAX_ADDR_BITS: usize = 44;
    let mut response_future = match Allocator.malloc_phys::<InputEvent>(MAX_ADDR_BITS) {
        Ok(mut buf) => {
            buf.write(InputEvent::uninit());
            Some(msg::recv_event(event_q, PhysBox::assume_init(buf)))
        },
        Err(_) => {
            let _ = writeln!(KernelWriter, "virtio-input: WARNING: failed to allocate an event buffer");
            None
        },
    };

    async move {
        if response_future.is_none() { return; };
        loop {
            match mem::replace(&mut response_future, None).unwrap().await {
                Ok(event) => {
                    // TODO
                    let _ = writeln!(KernelWriter, "virtio-input: event received: {:?}", event);

                    // Use the same buffer to await another event.
                    response_future = Some(msg::recv_event(event_q, event));
                },
                Err(e) => {
                    let _ = writeln!(KernelWriter, "virtio-input: ERROR: {e}");
                    return;
                }
            };
        }
    }
}

struct VirtIoDevice<'a> {
    details:    DeviceDetails<'a>,
    virtqueues: Vec<VirtQueue<'a>>,
}

// FIXME: Remove this debugging aid.
struct KernelWriter;

impl core::fmt::Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        unsafe {
            asm!(
                "svc 0xff00",
                in("x2") u64::from(u32::from(c)),
                options(nomem, preserves_flags, nostack)
            );
        }
        Ok(())
    }
}

#[repr(u32)]
enum QueueIndex {
    Event  = 0,
    Status = 1,
    Count = 2,
}

struct ConfigurationSpace<'a> {
    regs:   Volatile<&'a mut ConfigurationRegs>,
    legacy: bool,
}

impl<'a> ConfigurationSpace<'a> {
    // The number of bytes expected to be in the device's configuration space (i.e. referenced by `regs`).
    const SIZE: usize = mem::size_of::<ConfigurationRegs>();

    fn new(device_details: &'a mut DeviceDetails) -> Self {
        let legacy = device_details.legacy();
        let byte_slice = device_details.configuration_space();
        let regs = unsafe {
            assert_eq!(
                byte_slice as *mut [u8] as *mut u8 as usize % mem::align_of::<ConfigurationRegs>(),
                0,
                "configuration space is misaligned"
            );
            Volatile::new(&mut *(byte_slice as *mut [u8] as *mut ConfigurationRegs))
        };
        Self { regs, legacy }
    }

    // Reads a value from the configuration area and returns the number of bytes read.
    #[must_use]
    fn read(&mut self, select: u8, subsel: u8, value: &mut ConfigurationValue) -> u8 {
        self.regs.map_mut(|regs| &mut regs.select).write(select.to_device_endian(self.legacy));
        self.regs.map_mut(|regs| &mut regs.subsel).write(subsel.to_device_endian(self.legacy));
        let bitmap = unsafe { &mut value.bitmap };
        let size = u8::from_device_endian(self.regs.map(|regs| &regs.size).read(), self.legacy);
        for i in 0 .. size.into() {
            bitmap[i] = self.regs.map(|regs| unsafe { &regs.value.bitmap[i] }).read();
        }
        size
    }
}

#[repr(u8)]
enum ConfigSelect {
    // Unset    = 0x00,
    IdName   = 0x01,
    IdSerial = 0x02,
    IdDevIds = 0x03,
    PropBits = 0x10,
    EvBits   = 0x11,
    AbsInfo  = 0x12,
}

#[repr(C)]
struct ConfigurationRegs {
    select: u8,
    subsel: u8,
    size:   u8,
    _reserved: [u8; 5],
    value:  ConfigurationValue,
}

#[repr(C)]
union ConfigurationValue {
    string: [u8; 128], // The spec distinguishes this from `bitmap` only by using `char` instead of `u8`.
    bitmap: [u8; 128],
    abs:    AbsInfo,
    ids:    DevIds,
}

impl ConfigurationValue {
    fn uninit_string() -> Self {
        Self { string: [0; 128] }
    }

    fn uninit_bitmap() -> Self {
        Self { bitmap: [0; 128] }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct AbsInfo {
    // These are device-endian. Use the accessor methods instead.
    min:  u32,
    max:  u32,
    fuzz: u32,
    flat: u32,
    res:  u32,
}

impl AbsInfo {
    fn min(&self, legacy: bool) -> u32 {
        u32::from_device_endian(self.min, legacy)
    }

    fn max(&self, legacy: bool) -> u32 {
        u32::from_device_endian(self.max, legacy)
    }

    fn fuzz(&self, legacy: bool) -> u32 {
        u32::from_device_endian(self.fuzz, legacy)
    }

    fn flat(&self, legacy: bool) -> u32 {
        u32::from_device_endian(self.flat, legacy)
    }

    fn res(&self, legacy: bool) -> u32 {
        u32::from_device_endian(self.res, legacy)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct DevIds {
    // These are device-endian. Use the accessor methods instead.
    bustype: u16,
    vendor:  u16,
    product: u16,
    version: u16,
}

impl DevIds {
    fn bustype(&self, legacy: bool) -> u16 {
        u16::from_device_endian(self.bustype, legacy)
    }

    fn vendor(&self, legacy: bool) -> u16 {
        u16::from_device_endian(self.vendor, legacy)
    }

    fn product(&self, legacy: bool) -> u16 {
        u16::from_device_endian(self.product, legacy)
    }

    fn version(&self, legacy: bool) -> u16 {
        u16::from_device_endian(self.version, legacy)
    }
}

impl Default for DevIds {
    fn default() -> Self {
        Self {
            bustype: u16::max_value(),
            vendor:  u16::max_value(),
            product: u16::max_value(),
            version: u16::max_value(),
        }
    }
}

impl fmt::Display for DevIds {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // FIXME: The shown values will be wrong if this is a legacy device on a big-endian system.
        write!(f,
            "bustype = {:#x}, vendor = {:#x}, product = {:#x}, version = {:#x}",
            u16::from_le(self.bustype), u16::from_le(self.vendor), u16::from_le(self.product), u16::from_le(self.version),
        )
    }
}
