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

use core::ops::Deref;

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

impl<'a> Device<'a> {
    /// Retrieves the named device from the kernel.
    ///
    /// Multiple names can be specified. If they are, each is attempted in order, and the first one
    /// that matches a device that is present is returned.
    pub fn claim(names: &'_ [&str]) -> Option<Device<'a>> {
        // PERF: Run these system calls as a batch to avoid jumping into and out of the kernel
        //       repeatedly.
        for name in names {
            if let Some(device_addr) = libphoenix::syscall::device_claim(name) {
                return Some(Device {
                    contents: unsafe { &*(device_addr.get() as *const _) }
                });
            }
        }
        None
    }
}

impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        // FIXME: Use a system call to relinquish control of the device.
    }
}

impl<'a> Deref for Device<'a> {
    type Target = DeviceContents;

    fn deref(&self) -> &DeviceContents {
        self.contents
    }
}

/// Information about a device.
///
/// An object of this type should only be accessed through a [`Device`] object.
#[repr(C)]
#[derive(Debug)]
pub struct DeviceContents {
}
