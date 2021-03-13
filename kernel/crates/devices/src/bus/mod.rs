/* Copyright (c) 2020-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the buses that can be found in the device tree and a means to enumerate
//! them.

pub mod mmio;

use {
    alloc::collections::TryReserveError,
    core::{
        fmt,
        ptr::{read_volatile, write_volatile}
    },
    error::Error,
    i18n::Text,
    memory::phys::block::Mmio
};

use crate::DeviceTree;

/// Enumerates all the buses under the given level of the device tree.
pub fn enumerate(device_tree: &mut DeviceTree) -> Result<(), TryReserveError> {
    mmio::enumerate(device_tree)
    // TODO: pci::enumerate(device_tree);
}

/// Common functionality that all buses need to have.
pub trait Bus {
    /// Reserves a range of addresses on this bus as read-only. This guarantees exclusive access.
    /// For instance, although we only need read access, no one can get write access while these
    /// addresses are reserved.
    fn reserve_ro(&self, base: usize, size: usize) -> Result<ResourceRo, ReserveError>;

    /// Reserves a range of addresses on this bus as write-only. This guarantees exclusive access.
    /// For instance, although we only need write access, no one can get read access while these
    /// addresses are reserved.
    fn reserve_wo(&self, base: usize, size: usize) -> Result<ResourceWo, ReserveError>;

    /// Reserves a range of addresses on this bus as read-write. This guarantees exclusive access.
    fn reserve_rw(&self, base: usize, size: usize) -> Result<ResourceRw, ReserveError>;
}

/// A read-only range of addresses on a bus, with an interface for reading from them.
#[derive(Debug)]
pub struct ResourceRo {
    pub(crate) resource: Resource
}

/// A write-only range of addresses on a bus, with an interface for writing to them.
#[derive(Debug)]
pub struct ResourceWo {
    pub(crate) resource: Resource
}

/// A read-write range of addresses on a bus, with an interface for reading and writing.
#[derive(Debug)]
pub struct ResourceRw {
    pub(crate) resource: Resource
}

#[derive(Debug)]
pub(crate) enum Resource {
    Mmio(Mmio<u8>)
}

impl ResourceRo {
    /// Reads the value `offset` addresses beyond the start of this resource. Note that this
    /// doesn't work like pointer addition: if an MMIO block starts at `0xA0000`, then
    /// `read::<u16>::(8)` reads the `u16` from `0xA0008`, not from `0xA0010`.
    ///
    /// # Safety
    /// This function interacts directly with low-level system resources. Rust has no way to reason
    /// about its safety, so it is assumed to be unsafe.
    pub unsafe fn read<T: Copy>(&self, offset: usize) -> T {
        self.resource.read(offset)
    }
}

impl ResourceWo {
    /// Writes the given value `offset` addresses beyond the start of this resource. Note that this
    /// doesn't work like pointer addition: if an MMIO block starts at `0xA0000`, then
    /// `write::<u16>::(8, 0x076f)` will write `0x076f` to `0xA0008`, not to `0xA0010`.
    ///
    /// # Safety
    /// This function interacts directly with low-level system resources. Rust has no way to reason
    /// about its safety, so it is assumed to be unsafe.
    pub unsafe fn write<T: Copy>(&self, offset: usize, value: T) {
        self.resource.write(offset, value)
    }
}

impl ResourceRw {
    /// Reads the value `offset` addresses beyond the start of this resource. Note that this
    /// doesn't work like pointer addition: if an MMIO block starts at `0xA0000`, then
    /// `read::<u16>::(8)` reads the `u16` from `0xA0008`, not from `0xA0010`.
    ///
    /// # Safety
    /// This function interacts directly with low-level system resources. Rust has no way to reason
    /// about its safety, so it is assumed to be unsafe.
    pub unsafe fn read<T: Copy>(&self, offset: usize) -> T {
        self.resource.read(offset)
    }

    /// Writes the given value `offset` addresses beyond the start of this resource. Note that this
    /// doesn't work like pointer addition: if an MMIO block starts at `0xA0000`, then
    /// `write::<u16>::(8, 0x076f)` will write `0x076f` to `0xA0008`, not to `0xA0010`.
    ///
    /// # Safety
    /// This function interacts directly with low-level system resources. Rust has no way to reason
    /// about its safety, so it is assumed to be unsafe.
    pub unsafe fn write<T: Copy>(&self, offset: usize, value: T) {
        self.resource.write(offset, value)
    }
}

impl Resource {
    unsafe fn read<T: Copy>(&self, offset: usize) -> T {
        match *self {
            Self::Mmio(ref block) => read_volatile(block.index(offset) as *const _ as *const T)
        }
    }

    unsafe fn write<T: Copy>(&self, offset: usize, value: T) {
        match *self {
            Self::Mmio(ref block) => write_volatile(block.index(offset) as *mut T, value)
        }
    }
}

/// An error that can occur when trying to reserve an address range on a bus.
#[derive(Debug)]
pub struct ReserveError {
    bus_type: &'static str,
    base: usize,
    size: usize
}

impl Error for ReserveError {}

impl fmt::Display for ReserveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Text::CouldntReserveDeviceResource(self.bus_type, self.base, self.size))
    }
}
