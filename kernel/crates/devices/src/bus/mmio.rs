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

//! This module defines the "MMIO bus", which is really just the entire memory address space viewed
//! from the perspective of MMIO. The regular memory manager handles allocating ranges of addresses
//! on this bus.

use {
    alloc::vec::Vec,
    core::mem,

    libdriver::{BusType, Resource},

    memory::allocator::AllMemAlloc,

    super::{Bus, ReserveError},
    crate::DeviceTree
};

/// Enumerates any MMIO buses present at the given level of the device tree.
pub fn enumerate(device_tree: &mut DeviceTree) -> Result<(), ()> {
    match *device_tree {
        DeviceTree::Root { children: ref mut subtrees } => {
            subtrees.try_reserve(1)
                .map_err(|_| ())?;
            subtrees.push(DeviceTree::Mmio { bus: MmioBus, children: Vec::new() });
        },
        _ => {} // The MMIO bus is found only at the root.
    };
    Ok(())
}

/// The MMIO bus, spanning the entire memory address space.
#[derive(Debug)]
pub struct MmioBus;

impl Bus for MmioBus {
    fn reserve(&self, base: usize, size: usize) -> Result<Resource, ReserveError> {
        let block = AllMemAlloc.mmio_mut(base, size)
            .map_err(|_| ReserveError { bus_type: "mmio", base, size })?;
        let base: *mut u8 = block.base().as_virt_unchecked();
        let size = block.size();
        mem::forget(block);
        Ok(Resource {
            bus: BusType::Mmio,
            base: base as usize,
            size
        })
    }
}
