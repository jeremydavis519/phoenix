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

#![no_std]
#![deny(missing_docs)]

//! This library provides a generic API for communicating with VirtIO devices. Every VirtIO device
//! driver should link to this library to avoid duplicating code.

extern crate alloc;

use {
    alloc::vec::Vec,
    core::{
        convert::TryInto,
        fmt,
        mem,
        slice,
        sync::atomic::{AtomicU32, Ordering}
    },
    bitflags::bitflags,
    libdriver::{BusType, Device, Resource},
    libphoenix::syscall,
    self::virtqueue::{VirtQueue, DriverFlags}
};

pub mod virtqueue;

/// Initializes the given device.
pub fn init<'a>(
        device:            &'a Device,
        device_type:       u32,
        config_space_size: usize,
        queues_count:      u32,
        required_features: u64,
        optional_features: u64
) -> Result<DeviceDetails<'a>, VirtIoInitError> {
    let resources = device.resources();
    if resources.len() == 0 {
        return Err(VirtIoInitError::TooFewResources);
    }
    let resource = &resources[0];

    match resource.bus {
        BusType::Mmio => init_mmio(
            resource,
            device_type,
            config_space_size,
            queues_count,
            required_features,
            optional_features
        )
    }
}

fn init_mmio<'a>(
        resource:          &'a Resource,
        device_type:       u32,
        config_space_size: usize,
        queues_count:      u32,
        required_features: u64,
        optional_features: u64
) -> Result<DeviceDetails<'a>, VirtIoInitError> {
    assert_eq!(resource.bus, BusType::Mmio);
    if resource.size < 0x100 {
        return Err(VirtIoInitError::TooFewRegisters(0x100, resource.size));
    }
    if resource.size < 0x100 + config_space_size {
        return Err(VirtIoInitError::TooLittleConfigSpace(config_space_size, resource.size - 0x100))
    }
    let mut regs = MmioRegisters {
        slice: unsafe {
            slice::from_raw_parts_mut(
                resource.base as *mut u32,
                0x100 / mem::size_of::<u32>()
            )
        }
    };
    let configuration_space = unsafe {
        slice::from_raw_parts_mut(
            resource.base.wrapping_add(0x100) as *mut u8,
            resource.size - 0x100
        )
    };
    validate_mmio(&mut regs, device_type)?;

    // Reset and acknowledge the device.
    regs.set_status(DeviceStatus::empty())
        .or_status(DeviceStatus::ACKNOWLEDGE)
        .or_status(DeviceStatus::DRIVER);

    // Negotiate features.
    let device_features = regs.device_features();
    if required_features & !device_features != 0 {
        return Err(VirtIoInitError::MissingRequiredFeatures(required_features, device_features));
    }
    let mut features = device_features & (required_features | optional_features);
    regs.set_driver_features(features);

    let legacy = features & GenericFeatures::VERSION_1.bits() == 0;
    if !legacy { // Legacy devices didn't have the FEATURES_OK bit.
        regs.or_status(DeviceStatus::FEATURES_OK);
        if !regs.status().contains(DeviceStatus::FEATURES_OK) {
            // The device apparently didn't like that combination. Try again with just the required features.
            features = required_features;
            regs.set_driver_features(features)
                .or_status(DeviceStatus::FEATURES_OK);
            if !regs.status().contains(DeviceStatus::FEATURES_OK) {
                // The device isn't accepting any set of features that we can use.
                regs.or_status(DeviceStatus::FAILED);
                return Err(VirtIoInitError::FeatureNegotiationFailed);
            }
        }
    }

    // Initialize the virtqueues.
    let page_size;
    let mut virtqueues = Vec::new();
    if legacy {
        page_size = syscall::memory_page_size();
        regs.legacy_set_guest_page_size(
            page_size.try_into().expect("page size exceeds 32 bits")
        );
    } else {
        page_size = 0; // This isn't used here except in legacy devices.
    }
    for queue_index in 0 .. queues_count {
        regs.select_queue(queue_index);
        if legacy {
            assert_eq!(regs.legacy_queue_page_number(), 0, "virtqueue {} already in use", queue_index);
        } else {
            assert!(!regs.queue_ready(), "virtqueue {} already in use", queue_index);
        }
        let max_queue_len = regs.queue_len_max();
        if max_queue_len == 0 {
            // Assume for now that this virtqueue isn't necessary. If the driver needs this queue, it can
            // panic after we finish.
            continue;
        }
        let queue_len = u32::min(max_queue_len, 0x8000);
        // TODO: Allow the driver to specify the DriverFlags for each virtqueue.
        let queue = VirtQueue::new(resource, features, legacy, queue_index, queue_len as u16, DriverFlags::empty());
        regs.set_queue_len(queue_len);
        if legacy {
            regs.legacy_set_device_ring_align(
                VirtQueue::LEGACY_DEVICE_RING_ALIGN.try_into().expect("device ring alignment exceeds 32 bits")
            );
            let page_number = (queue.descriptors_addr_phys() / page_size).try_into()
                .expect("virtqueue address is too high");
            regs.legacy_set_queue_page_number(page_number);
        } else {
            regs.set_queue_descriptor_area(queue.descriptors_addr_phys().try_into().unwrap());
            regs.set_queue_driver_area(queue.driver_ring_addr_phys().try_into().unwrap());
            regs.set_queue_device_area(queue.device_ring_addr_phys().try_into().unwrap());
            regs.set_queue_ready(true);
        }
        virtqueues.push(queue);
    }

    regs.or_status(DeviceStatus::DRIVER_OK);

    Ok(DeviceDetails {
        legacy,
        features,
        configuration_space,
        virtqueues
    })
}

fn validate_mmio<'a>(
        regs:              &mut MmioRegisters<'a>,
        device_type:       u32
) -> Result<(), VirtIoInitError> {
    const MAGIC_NUMBER:    u32 = 0x74726976; // Little-endian "virt"
    const CURRENT_VERSION: u32 = 1;

    let found_magic_number = regs.magic_number();
    if found_magic_number != MAGIC_NUMBER {
        return Err(VirtIoInitError::WrongMagicNumber(MAGIC_NUMBER, found_magic_number));
    }
    let version = regs.version();
    if version < 1 || version > CURRENT_VERSION {
        return Err(VirtIoInitError::UnsupportedVersion(CURRENT_VERSION, version));
    }
    let found_device_type = regs.device_id();
    if found_device_type != device_type {
        return Err(VirtIoInitError::WrongDeviceType(device_type, found_device_type));
    }

    Ok(())
}

struct MmioRegisters<'a> {
    slice: &'a mut [u32]
}

impl<'a> MmioRegisters<'a> {
    fn magic_number(&mut self) -> u32 {
        unsafe { u32::from_le((&self.slice[0x00] as *const u32).read_volatile()) }
    }

    fn version(&mut self) -> u32 {
        unsafe { u32::from_le((&self.slice[0x01] as *const u32).read_volatile()) }
    }

    fn device_id(&mut self) -> u32 {
        unsafe { u32::from_le((&self.slice[0x02] as *const u32).read_volatile()) }
    }

    fn vendor_id(&mut self) -> u32 {
        unsafe { u32::from_le((&self.slice[0x03] as *const u32).read_volatile()) }
    }

    fn device_features(&mut self) -> u64 {
        unsafe {
            (&mut self.slice[0x05] as *mut u32).write_volatile(FeaturesSelection::Low as u32);
            let low = u32::from_le((&self.slice[0x04] as *const u32).read_volatile());

            (&mut self.slice[0x05] as *mut u32).write_volatile(FeaturesSelection::High as u32);
            let high = u32::from_le((&self.slice[0x04] as *const u32).read_volatile());

            u64::from(low) | (u64::from(high) << 32)
        }
    }

    fn set_driver_features(&mut self, features: u64) -> &mut Self {
        unsafe {
            (&mut self.slice[0x09] as *mut u32).write_volatile(FeaturesSelection::Low as u32);
            (&mut self.slice[0x08] as *mut u32).write_volatile((features as u32).to_le());

            (&mut self.slice[0x09] as *mut u32).write_volatile(FeaturesSelection::High as u32);
            (&mut self.slice[0x08] as *mut u32).write_volatile(((features >> 32) as u32).to_le());

            self
        }
    }

    fn legacy_set_guest_page_size(&mut self, page_size: u32) -> &mut Self {
        unsafe { (&mut self.slice[0x0a] as *mut u32).write_volatile(page_size.to_le()); }
        self
    }

    fn select_queue(&mut self, queue_index: u32) -> &mut Self {
        unsafe { (&mut self.slice[0x0c] as *mut u32).write_volatile(queue_index.to_le()); }
        self
    }

    fn queue_len_max(&mut self) -> u32 {
        unsafe { u32::from_le((&self.slice[0x0d] as *const u32).read_volatile()) }
    }

    fn set_queue_len(&mut self, len: u32) -> &mut Self {
        unsafe { (&mut self.slice[0x0e] as *mut u32).write_volatile(len.to_le()); }
        self
    }

    fn legacy_set_device_ring_align(&mut self, align: u32) -> &mut Self {
        unsafe { (&mut self.slice[0x0f] as *mut u32).write_volatile(align.to_le()); }
        self
    }

    fn legacy_queue_page_number(&mut self) -> u32 {
        unsafe { u32::from_le((&self.slice[0x10] as *const u32).read_volatile()) }
    }

    fn legacy_set_queue_page_number(&mut self, page_number: u32) -> &mut Self {
        unsafe { (&mut self.slice[0x10] as *mut u32).write_volatile(page_number.to_le()); }
        self
    }

    fn queue_ready(&mut self) -> bool {
        match unsafe { u32::from_le((&self.slice[0x11] as *const u32).read_volatile()) } {
            0 => false,
            1 => true,
            x => panic!("invalid value found in QueueReady: {}", x)
        }
    }

    fn set_queue_ready(&mut self, ready: bool) -> &mut Self {
        unsafe { (&mut self.slice[0x11] as *mut u32).write_volatile(u32::to_le(if ready { 1 } else { 0 })); }
        self
    }

    fn queue_notify(&mut self, notification: u32) -> &mut Self {
        // NOTE: If VIRTIO_F_NOTIFICATION_DATA has been negotiated, `notification` contains more than
        //       just a queue index.
        unsafe { (&mut self.slice[0x14] as *mut u32).write_volatile(notification.to_le()); }
        self
    }

    fn interrupt_status(&mut self) -> Interrupts {
        unsafe { Interrupts::from_bits_unchecked((&self.slice[0x18] as *const u32).read_volatile()) }
    }

    fn acknowledge_interrupt(&mut self, interrupts: Interrupts) -> &mut Self {
        unsafe { (&mut self.slice[0x19] as *mut u32).write_volatile(interrupts.bits()); }
        self
    }

    fn status(&mut self) -> DeviceStatus {
        unsafe { DeviceStatus::from_bits_unchecked((&self.slice[0x1c] as *const u32).read_volatile()) }
    }

    fn set_status(&mut self, status: DeviceStatus) -> &mut Self {
        unsafe { (&mut self.slice[0x1c] as *mut u32).write_volatile(status.bits()); }
        self
    }

    fn or_status(&mut self, mut status: DeviceStatus) -> &mut Self {
        status |= self.status();
        self.set_status(status)
    }

    fn set_queue_descriptor_area(&mut self, phys_addr: u64) -> &mut Self {
        unsafe {
            (&mut self.slice[0x20] as *mut u32).write_volatile((phys_addr as u32).to_le());
            (&mut self.slice[0x21] as *mut u32).write_volatile(((phys_addr >> 32) as u32).to_le());
        }
        self
    }

    fn set_queue_driver_area(&mut self, phys_addr: u64) -> &mut Self {
        unsafe {
            (&mut self.slice[0x24] as *mut u32).write_volatile((phys_addr as u32).to_le());
            (&mut self.slice[0x25] as *mut u32).write_volatile(((phys_addr >> 32) as u32).to_le());
        }
        self
    }

    fn set_queue_device_area(&mut self, phys_addr: u64) -> &mut Self {
        unsafe {
            (&mut self.slice[0x28] as *mut u32).write_volatile((phys_addr as u32).to_le());
            (&mut self.slice[0x29] as *mut u32).write_volatile(((phys_addr >> 32) as u32).to_le());
        }
        self
    }

    fn config_generation(&mut self) -> u32 {
        // This is probably little-endian, but it's an opaque value, so it doesn't matter. The
        // only meaningful operation on this value is a test for equality with another value
        // from the same register.
        unsafe { (*(&mut self.slice[0x3f] as *mut u32 as *mut AtomicU32)).load(Ordering::Acquire) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
enum FeaturesSelection {
    Low  = u32::to_le(0),
    High = u32::to_le(1)
}

bitflags! {
    /// The standard set of features that apply to all devices.
    pub struct GenericFeatures: u64 {
        /// Whether the device needs to send a notification when it runs out of buffers, even if
        /// notifications are suppressed.
        const NOTIFY_ON_EMPTY     = 0x0000_0000_0100_0000;

        /// Whether the device accepts arbitrary descriptor layouts.
        const ANY_LAYOUT          = 0x0000_0000_0800_0000;

        /// Whether the driver can use indirect descriptors.
        const RING_INDIRECT_DESC  = 0x0000_0000_1000_0000;

        /// Enables the used_event and avail_event fields.
        const RING_EVENT_INDEX    = 0x0000_0000_2000_0000;

        /// Indicates that the device is not a legacy device.
        const VERSION_1           = 0x0000_0001_0000_0000;

        /// Indicates that the device can be used on a platform that limits or translates its memory
        /// accesses.
        const ACCESS_PLATFORM     = 0x0000_0002_0000_0000;

        /// Indicates support for the packed virtqueue layout.
        const RING_PACKED         = 0x0000_0004_0000_0000;

        /// Indicates that the device uses all buffers in the same order in which they're made
        /// available.
        const IN_ORDER            = 0x0000_0008_0000_0000;

        /// Indicates that the device may be real hardware and therefore needs memory accesses to be
        /// ordered according to the platform's normal requirements.
        ///
        /// If this is not negotiated, the device is assumed to be implemented in software, which
        /// allows weaker memory barriers to be used instead.
        const ORDER_PLATFORM      = 0x0000_0010_0000_0000;

        /// Indicates that the device supports Single Root I/O Virtualization.
        const SINGLE_ROOT_IO_VIRT = 0x0000_0020_0000_0000;

        /// Indicates that the driver will provide extra data in its device notifications.
        const NOTIFICATION_DATA   = 0x0000_0040_0000_0000;
    }
}

bitflags! {
    struct Interrupts: u32 {
        const USED_BUFFER    = u32::to_le(0x0000_0001);
        const CONFIG_CHANGED = u32::to_le(0x0000_0002);
    }
}

bitflags! {
    struct DeviceStatus: u32 {
        const ACKNOWLEDGE = u32::to_le(0x01); // OS has noticed the device
        const DRIVER      = u32::to_le(0x02); // OS knows how to drive the device
        const DRIVER_OK   = u32::to_le(0x04); // Driver is ready
        const FEATURES_OK = u32::to_le(0x08); // Driver has acknowledged the features it understands
        const NEEDS_RESET = u32::to_le(0x40); // Device has experienced an error and needs to be reset
        const FAILED      = u32::to_le(0x80); // OS has given up on the device
    }
}

/// A collection of VirtIO-specific information about a device, returned by [`init`].
#[derive(Debug)]
pub struct DeviceDetails<'a> {
    legacy:              bool,
    features:            u64,
    configuration_space: &'a mut [u8],
    virtqueues:          Vec<VirtQueue<'a>>
}

impl<'a> DeviceDetails<'a> {
    /// Indicates whether this is a legacy device.
    ///
    /// A legacy device is defined as one that adheres to a version of the VirtIO specification
    /// before version 1.0.
    pub fn legacy(&self) -> bool {
        self.legacy
    }

    /// Returns a slice containing the device's configuration space.
    ///
    /// The configuration space's layout depends on the device type, so all we can do is return a
    /// byte slice.
    pub fn configuration_space(&mut self) -> &mut [u8] {
        self.configuration_space
    }

    /// Transfers ownership of the device's virtqueues. This can only be done once.
    pub fn virtqueues(&mut self) -> Vec<VirtQueue<'a>> {
        mem::replace(&mut self.virtqueues, Vec::new())
    }
}

fn notify_device<'a>(resource: &'a Resource, notification: u32) {
    match resource.bus {
        BusType::Mmio => notify_mmio(resource, notification)
    }
}

fn notify_mmio<'a>(resource: &'a Resource, notification: u32) {
    assert_eq!(resource.bus, BusType::Mmio);
    assert!(resource.size >= 0x100);
    let mut regs = MmioRegisters {
        slice: unsafe {
            slice::from_raw_parts_mut(
                resource.base as *mut u32,
                0x100 / mem::size_of::<u32>()
            )
        }
    };

    regs.queue_notify(notification);
}

/// Defines how to convert an integer from "device-endian" to the CPU's endianness.
///
/// This is necessary because the VirtIO specification used to say that a device always used the
/// CPU's endianness for almost everything, but now absolutely everything is little-endian. Since we
/// handle both legacy and non-legacy devices, we must be prepared for both cases. This trait lets
/// us avoid repeating the same logic over and over.
pub trait DeviceEndian {
    /// Converts a number from the device's endianness to the CPU's endianness.
    fn from_device_endian(val: Self, legacy: bool) -> Self;

    /// Converts a number from the CPU's endianness to the device's endianness.
    fn to_device_endian(self, legacy: bool) -> Self;
}

macro_rules! impl_device_endian {
    ($t:ty) => {
        impl DeviceEndian for $t {
            fn from_device_endian(val: $t, legacy: bool) -> $t {
                if legacy {
                    val
                } else {
                    <$t>::from_le(val)
                }
            }

            fn to_device_endian(self, legacy: bool) -> $t {
                if legacy {
                    self
                } else {
                    self.to_le()
                }
            }
        }
    };
}

impl_device_endian!(u8);
impl_device_endian!(i8);
impl_device_endian!(u16);
impl_device_endian!(i16);
impl_device_endian!(u32);
impl_device_endian!(i32);
impl_device_endian!(u64);
impl_device_endian!(i64);

/// An error that might occur when trying to initialize a VirtIO device.
#[derive(Debug)]
pub enum VirtIoInitError {
    /// The given device owned too few resources.
    TooFewResources,
    /// Too few registers for this to be a valid VirtIO device.
    TooFewRegisters(usize, usize),
    /// Less configuration space available than the driver requested.
    TooLittleConfigSpace(usize, usize),
    /// The device doesn't have the right magic number to be a VirtIO device.
    WrongMagicNumber(u32, u32),
    /// The device uses a version of the VirtIO specification that we don't support.
    UnsupportedVersion(u32, u32),
    /// The device isn't of the type (e.g. GPU, network card, block device) that we expected.
    WrongDeviceType(u32, u32),
    /// The device doesn't support all of the features that the driver requires.
    MissingRequiredFeatures(u64, u64),
    /// The device didn't accept our requested set of features.
    FeatureNegotiationFailed
}

impl fmt::Display for VirtIoInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::TooFewResources
                => write!(f, "device owns too few resources"),
            Self::TooFewRegisters(expected, actual)
                => write!(f, "device has too few registers: expected {}, found {}", expected, actual),
            Self::TooLittleConfigSpace(expected, actual)
                => write!(f, "device has too little configuration space: expected {}, found {}", expected, actual),
            Self::WrongMagicNumber(expected, actual)
                => write!(f, "magic number not found: expected {:#x}, found {:#x}", expected, actual),
            Self::UnsupportedVersion(expected, actual)
                => write!(f, "VirtIO version {} not supported (we only support up to version {})", actual, expected),
            Self::WrongDeviceType(expected, actual)
                => write!(f, "wrong device type found: expected {}, found {}", expected, actual),
            Self::MissingRequiredFeatures(required, found)
                => write!(f, "driver requires feature set {:#x}, but device only supports {:#x}", required, found),
            Self::FeatureNegotiationFailed
                => write!(f, "feature negotiation failed")
        }
    }
}

/// An error that might occur while communicating with a VirtIO device.
#[derive(Debug)]
pub struct VirtIoError {
    desc: &'static str
}

impl VirtIoError {
    fn new(desc: &'static str) -> VirtIoError {
        VirtIoError { desc }
    }
}

impl fmt::Display for VirtIoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc)
    }
}
