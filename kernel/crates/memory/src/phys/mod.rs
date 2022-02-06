/* Copyright (c) 2018-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module contains the part of the memory manager that deals directly with physical memory.
//! It defines physical pointers (as opposed to Rust's pointers, which are all virtual), blocks of
//! physical memory (which may each be cut into smaller chunks for virtual memory), and the
//! kernel's heap (using physical addresses).

#[cfg(not(feature = "unit-test"))]
use i18n::Text;

pub(crate) mod map;
pub use self::map::RegionType;

pub mod ptr;
pub mod block;
pub(crate) mod heap;

#[cfg(target_arch = "aarch64")]
lazy_static! {
    unsafe {
        /// The number of bits that a physical address can use
        pub static ref PHYS_ADDR_BITS: u8 = {
            let flags: u64;
            asm!(
                "mrs {}, ID_AA64MMFR0_EL1",
                out(reg) flags,
                options(nomem, nostack, preserves_flags)
            );
            match flags & 0xf {
                0b0000 => 32,
                0b0001 => 36,
                0b0010 => 40,
                0b0011 => 42,
                0b0100 => 44,
                0b0101 => 48,
                0b0110 => 52,
                _ => panic!("{}", Text::Aarch64UnrecognizedPhysAddrSize(flags))
            }
        };
    }
}
#[cfg(target_arch = "x86_64")]
lazy_static! {
    unsafe {
        /// The number of bits that a physical address can use
        pub static ref PHYS_ADDR_BITS: u8 = {
            // Check the memory map. The number of physical address bits we support will be
            // the number of bits required for the highest available or hotpluggable byte.
            // FIXME: This doesn't work, since the `MemoryMap` code is what depends on the
            // value we're trying to calculate. We'll need to access the lower-level
            // methods. For now, we just return 63. (64 would work, but it causes overflow
            // when calculating the maximum address.)
            /*let mut max_address: usize = 0;
            let map = match map::memory_map() {
                Ok(map) => map,
                Err(()) => panic!("{}", Text::MemoryMapNotRetrieved)
            };
            for region in map.try_read().unwrap().present_regions() {
                let address = region.base.wrapping_add(region.size).wrapping_sub(1);
                if address > max_address {
                    max_address = address;
                }
            }
            (mem::size_of_val(&max_address) * 8 - (max_address.leading_zeros() as usize)) as u8*/
            63
        };
    }
}
