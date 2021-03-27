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

//! This crate is a sort of "standard library" of functions and types that are generally useful to
//! drivers but not to other programs. It is written specifically for the Phoenix operating system.

#![no_std]
#![deny(warnings, missing_docs)]

// FIXME: These `cfg` lines are here only to allow compiling on an x86-64 host.
#[cfg(target_arch = "aarch64")]
use core::{
    mem,
    num::NonZeroUsize,
    ops::Deref,
    slice
};
#[cfg(not(target_arch = "aarch64"))]
use core::{mem};

#[cfg(target_arch = "aarch64")]
/// Represents a physical device owned by this process.
#[derive(Debug)]
pub struct Device<'a> {
    /// The information about the device that we received from the kernel.
    ///
    /// This is really just an implementation detail and can be invisible in practice, since a
    /// `Device` dereferences to this member.
    ///
    /// # Example
    /// ```
    /// # let device = Device { contents: &DeviceContents {} };
    /// // (Given that `device` has type `Device`)
    /// assert_eq!(device.contents as *const DeviceContents, &*device as *const DeviceContents);
    /// ```
    pub contents: &'a DeviceContents
}

#[cfg(target_arch = "aarch64")]
impl<'a> Device<'a> {
    /// Retrieves the named device from the kernel.
    ///
    /// Note that the "name" of a device varies depending on the bus it's found on. See the Phoenix
    /// wiki for details on how devices are named on each bus.
    // TODO: Change this API to return a `SystemCall` object. Such an object will represent a chain
    //       of system calls to be made all at once, using an internal DSL for control flow, to
    //       avoid having to return to userspace between them.
    pub async fn claim(name: &str) -> Option<Device<'a>> {
        match NonZeroUsize::new(libphoenix::syscall::device_claim(name).await) {
            Some(device_addr) => Some(Device {
                    contents: unsafe { &*(device_addr.get() as *const _) }
                }),
            None => None
        }
    }

    /// Returns the device's slice of resource descriptors.
    pub fn resources(&self) -> &[Resource] {
        unsafe {
            slice::from_raw_parts(
                &self.contents.resources as *const [Resource; 0] as *const Resource,
                self.contents.resources_count
            )
        }
    }
}

#[cfg(target_arch = "aarch64")]
impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        // FIXME: Use a system call to relinquish control of the device.
    }
}

#[cfg(target_arch = "aarch64")]
impl<'a> Deref for Device<'a> {
    type Target = DeviceContents;

    fn deref(&self) -> &DeviceContents {
        self.contents
    }
}

/// Information about a device.
///
/// An object of this type should only be accessed through a [`Device`] object. Any object of this
/// type returned by the kernel will be write-protected.
#[repr(C)]
#[derive(Debug)]
pub struct DeviceContents {
    /// The number of elements in `resources`.
    pub resources_count: usize,

    /// Describes the resources owned by the device.
    ///
    /// (This array has a size of 0 only because of Rust's limitations regarding DSTs. The real size
    /// is stored in `resources_count`.
    pub resources: [Resource; 0]
}

impl DeviceContents {
    /// Returns the size of a `DeviceContents` object with the given number of resources.
    pub const fn size_with_resources(resources_count: usize) -> usize {
        mem::size_of::<DeviceContents>() + resources_count * mem::size_of::<Resource>()
    }
}

/// A description of a resource (i.e. a block of registers) owned by a device.
#[repr(C)]
#[derive(Debug)]
pub struct Resource {
    /// The bus on which this resource can be found.
    pub bus: BusType,

    /// The lowest address on the bus that this resource uses.
    pub base: usize,

    /// The number of addresses that this resource uses.
    pub size: usize
}

/// An enumeration of bus types for the purpose of locating resources.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    /// Memory-mapped I/O (registers are accessed just like RAM)
    Mmio = 0
}
