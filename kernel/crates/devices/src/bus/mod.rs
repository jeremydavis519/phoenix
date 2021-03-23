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
    core::fmt,
    libdriver::Resource,
    error::Error,
    i18n::Text,
    crate::DeviceTree
};

/// Enumerates all the buses under the given level of the device tree.
pub fn enumerate(device_tree: &mut DeviceTree) -> Result<(), ()> {
    mmio::enumerate(device_tree)
    // TODO: pci::enumerate(device_tree);
}

/// Common functionality that all buses need to have.
pub trait Bus {
    /// Reserves a range of addresses on this bus. This guarantees exclusive access by preventing
    /// two devices to use the same addresses.
    fn reserve(&self, base: usize, size: usize) -> Result<Resource, ReserveError>;
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
