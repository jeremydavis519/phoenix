/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

// TODO: Move most of this module to its own crate and rewrite it to be able to traverse the DTB at
//       any time and query it for specific information. Then, in this module, use that interface to
//       look for and parse the memory nodes. The new interface should be built around iterators:
//       `DTB.root_nodes()`, `node.children()`, and `node.properties()`.

//! This module provides a means to parse the device tree block (DTB) that the Linux kernel requires
//! its bootloaders to provide. It is only used on systems where we rely on a Linux-compatible
//! bootloader (such as the Raspberry Pi) and there's no other way to get some piece of information.
//! Other forms of discovery, such as ACPI and asking the hardware, are generally preferred when they
//! are available. (For instance, we, like the Linux kernel, don't require that all the PCI devices be
//! enumerated in the DTB, and we'll just ignore those records if we find them.)

use core::ffi::c_void;
use core::num::NonZeroUsize;
use core::slice;
use shared::ffi::{Be, CStrRef, Endian};
use super::{MemoryMap, RegionType};

/// Stores all the data we've retrived from the DTB in an easily retrievable format.
pub struct ParsedDtb {
    pub memory_map: MemoryMap
}

lazy_static! {
    unsafe {
        pub static ref PARSED_DTB: Option<ParsedDtb> = parse(super::BOOT_INFO.dtb());
    }
}

unsafe fn parse(dtb_start: *const c_void) -> Option<ParsedDtb> {
    if dtb_start.is_null() {
        return None;
    }

    let header = &*(dtb_start as *const DTBHeader);
    if !is_dtb(header) {
        return None;
    }

    try_parse_v11(header)
}

/// The header of the DTB.
#[repr(C)]
struct DTBHeader {
    magic_number: Be<u32>,            // Identifies this block of memory as a DTB
    dtb_size: Be<u32>,                // Total size of the whole DTB
    dt_struct_offset: Be<u32>,        // Offset to the flattened DT
    dt_strings_offset: Be<u32>,       // Offset to the DT's strings
    reserved_mem_map_offset: Be<u32>, // Offset to the map of reserved memory
    version: Be<u32>,                 // The version of the DTB format
    earliest_compat_version: Be<u32>, // The earliest version with which this version is backwards-compatible

    // Version 0x02 and up
    phys_boot_cpu_id: Be<u32>,        // The physical ID of the boot CPU

    // Version 0x03 and up
    dt_strings_size: Be<u32>,         // The size of the block pointed to by dt_strings_offset

    // Version 0x11 and up
    dt_struct_size: Be<u32>           // The size of the block pointed to by dt_struct_offset
}

/// Used by the DTB to represent a single region of reserved memory.
#[repr(C)]
struct DTBRsvMemEntry {
    address: Be<u64>,
    size:    Be<u64>
}

impl DTBRsvMemEntry {
    pub fn is_null(&self) -> bool {
        self.address.into_native() == 0 && self.size.into_native() == 0
    }
}

const OF_DT_BEGIN_NODE: u32 = 0x1; // Marks the beginning of a device
const OF_DT_END_NODE:   u32 = 0x2; // Marks the end of a device
const OF_DT_PROP:       u32 = 0x3; // Marks one of a device's properties
const OF_DT_NOP:        u32 = 0x4; // Should be ignored
const OF_DT_END:        u32 = 0x9; // Marks the end of the device tree

// Parses the DTB if it's compatible with any format version from 0x01 to 0x11.
unsafe fn try_parse_v11(header: &DTBHeader) -> Option<ParsedDtb> {
    // Version 0x10 isn't backwards-compatible with version 0x03, but version 0x03 is forwards-compatible
    // with version 0x10. So the general logic can be written for 0x11, as long as we're careful about the
    // header fields that were added with new versions.
    if header.version.into_native() < 0x01 || header.earliest_compat_version.into_native() > 0x11 {
        return None;
    }

    let mut parsed_dtb = ParsedDtb { memory_map: MemoryMap::new() };

    // Parse the device tree. There should only be one root device, which is at offset 0 in the structure.
    if verify_first_token(header).is_err() {
        return None;
    }
    match parse_node(header, 0, 0, 0, &mut parsed_dtb) {
        Err(()) => return None,
        Ok(offset) => {
            let header_addr = header as *const _ as usize;
            let tree_ptr = (header_addr + header.dt_struct_offset.into_native() as usize) as *const Be<u32>;
            if verify_last_token(tree_ptr.add(offset)).is_err() {
                return None;
            }
        }
    }

    // Reserve any memory the DTB says to reserve.
    let header_addr = header as *const _ as usize;
    let mut next_rsv_mem_entry = (header_addr + header.reserved_mem_map_offset.into_native() as usize) as *const DTBRsvMemEntry;
    while !(&*next_rsv_mem_entry).is_null() {
        let entry = &*next_rsv_mem_entry;
        let address = entry.address.into_native() as usize;
        let size = entry.size.into_native() as usize;
        if let Some(size) = NonZeroUsize::new(size) {
            if parsed_dtb.memory_map.remove_region(address, size).is_err() {
                return None;
            }
        }
        next_rsv_mem_entry = next_rsv_mem_entry.offset(1);
    }

    Some(parsed_dtb)
}

// Parses a node in the DTB, and all of its children.
// Returns `Ok(offset)`, where `offset` is the offset of the next token, or `Err` if we failed to
// parse the tree.
unsafe fn parse_node<'a>(header: &'a DTBHeader, token_offset: usize, parent_address_cells: usize, parent_size_cells: usize, parsed_dtb: &mut ParsedDtb)
        -> Result<usize, ()> {
    let header_addr = header as *const _ as usize;
    let tree_ptr = (header_addr + header.dt_struct_offset.into_native() as usize) as *const Be<u32>;

    let mut offset = token_offset;

    // The opening token is already known to be OF_DT_BEGIN_NODE, so skip it.
    offset += 1;

    // Skip the node's name (or path). We don't need it. Also skip the null terminator.
    let name_ptr = tree_ptr.add(offset) as *const u8;
    let name_offset = CStrRef::from_ptr(name_ptr).len() + 1;
    // The next field is aligned on a 4-byte boundary.
    offset += (name_offset + 3) / 4;

    // Take note of any properties we might find interesting.
    let mut prop_address_cells: usize = 0;
    let mut prop_size_cells: usize = 0;
    let mut prop_device_type: Option<&[u8]> = None;
    let mut prop_reg: Option<&[Be<u32>]> = None;
    let mut prop_hotpluggable: bool = false;
    while (*tree_ptr.add(offset)).into_native() == OF_DT_NOP {
        offset += 1;
    }
    while (*tree_ptr.add(offset)).into_native() == OF_DT_PROP {
        offset += 1;
        let property_size = (*tree_ptr.add(offset)).into_native() as usize;
        offset += 1;
        let property_name_offset = (*tree_ptr.add(offset)).into_native();
        offset += 1;
        match get_property_name(header, property_name_offset).as_bytes() {
            b"#address-cells" => {
                prop_address_cells = be_bytes_to_usize(slice::from_raw_parts(tree_ptr.add(offset) as *const Be<u8>, property_size))?;
            },

            b"#size-cells" => {
                prop_size_cells = be_bytes_to_usize(slice::from_raw_parts(tree_ptr.add(offset) as *const Be<u8>, property_size))?;
            },

            b"device_type" => {
                let string_ptr = tree_ptr.add(offset) as *const u8;
                let c_str = CStrRef::from_ptr(string_ptr);
                prop_device_type = Some(slice::from_raw_parts(string_ptr, c_str.len_capped(property_size)));
            },

            b"reg" => {
                if parent_address_cells != 0 && parent_size_cells != 0 {
                    // Make a slice of u32s with as many whole address/size pairs as possible.
                    // If the property size is poorly defined (i.e. not a multiple of the size of one pair), round down.
                    let parent_total_cells = parent_address_cells + parent_size_cells;
                    let property_cells = (property_size + 3) / 4 / parent_total_cells * parent_total_cells;
                    prop_reg = Some(slice::from_raw_parts(tree_ptr.add(offset), property_cells));
                }
            },

            b"hotpluggable" => {
                // This property doesn't take a value.
                prop_hotpluggable = true;
            },

            _ => {}
        }
        // Align to a 4-byte boundary after the property value.
        offset += (property_size + 3) / 4;

        while (*tree_ptr.add(offset)).into_native() == OF_DT_NOP {
            offset += 1;
        }
    }

    // Depending on the device type, we might want to save some of its properties.
    match prop_device_type {
        Some(b"memory") => {
            if let Some(prop_reg) = prop_reg {
                add_phys_mem_regions(prop_reg, parent_address_cells, parent_size_cells, prop_hotpluggable, &mut parsed_dtb.memory_map)?;
            }
        },

        Some(b"cpu") => {
            // TODO
        },

        _ => {}
    };

    // Parse any children this device has.
    while (*tree_ptr.add(offset)).into_native() == OF_DT_BEGIN_NODE {
        offset = parse_node(header, offset, prop_address_cells, prop_size_cells, parsed_dtb)?;
        while (*tree_ptr.add(offset)).into_native() == OF_DT_NOP {
            offset += 1;
        }
    }

    if (*tree_ptr.add(offset)).into_native() == OF_DT_END_NODE {
        offset += 1;
        Ok(offset)
    } else { // Any other token is invalid in this position.
        Err(())
    }
}

// Returns `Ok` iff the first token in the DT is OF_DT_BEGIN_NODE.
unsafe fn verify_first_token(header: &DTBHeader) -> Result<(), ()> {
    let header_addr = header as *const _ as usize;
    let tree_start = (header_addr as *const u8).add(header.dt_struct_offset.into_native() as usize) as *const Be<u32>;
    let mut offset = 0;
    while (*tree_start.add(offset)).into_native() == OF_DT_NOP {
        offset += 1;
    }
    if (*tree_start.add(offset)).into_native() == OF_DT_BEGIN_NODE {
        Ok(())
    } else {
        Err(())
    }
}

// Returns `Ok` iff the given token is OF_DT_END.
unsafe fn verify_last_token(token: *const Be<u32>) -> Result<(), ()> {
    let mut offset = 0;
    while (*token.add(offset)).into_native() == OF_DT_NOP {
        offset += 1;
    }
    if (*token.add(offset)).into_native() == OF_DT_END {
        Ok(())
    } else {
        Err(())
    }
}

// Returns the given property's name as a C string.
unsafe fn get_property_name(header: &DTBHeader, prop_name_offset: u32) -> CStrRef {
    let header_addr = header as *const _ as usize;
    let strings_ptr = (header_addr as *const u8).add(header.dt_strings_offset.into_native() as usize);

    let string_start = strings_ptr.add(prop_name_offset as usize);

    CStrRef::from_ptr(string_start)
}

// Converts a sequence of `u8`s in big-endian order into a `usize`.
// Also converts the property from big-endian to the system endianness.
// If the sequence is too long to fit in a `usize`, it returns `Err`.
fn be_bytes_to_usize(bytes: &[Be<u8>]) -> Result<usize, ()> {
    if bytes.len() <= core::mem::size_of::<usize>() {
        let mut full = 0;
        for piece in bytes {
            full = (full << 8) | piece.into_native() as usize;
        }
        Ok(full)
    } else {
        Err(())
    }
}

// Determines whether this is actually a DTB.
fn is_dtb(header: &DTBHeader) -> bool {
    header.magic_number.into_native() == 0xd00dfeed
}

// Adds memory regions to the physical memory map based on the given "reg" property, taken from one of the DT's
// memory nodes.
unsafe fn add_phys_mem_regions(prop_reg: &[Be<u32>], parent_address_cells: usize, parent_size_cells: usize, hotpluggable: bool,
        memory_map: &mut MemoryMap) -> Result<(), ()> {
    let entry_cells = parent_address_cells + parent_size_cells; // The number of 32-bit cells used to describe one region
    let prop_reg_bytes = slice::from_raw_parts(prop_reg as *const _ as *const Be<u8>, prop_reg.len() * 4);

    let mut index = 0;
    while index < prop_reg.len() {
        let address_end = index + parent_address_cells;
        let size_end = address_end + parent_size_cells;
        let address = be_bytes_to_usize(&prop_reg_bytes[index * 4 .. address_end * 4])?;
        let size = be_bytes_to_usize(&prop_reg_bytes[address_end * 4 .. size_end * 4])?;

        if let Some(size) = NonZeroUsize::new(size) {
            memory_map.add_region(address, size, RegionType::Ram, hotpluggable)?;
        }

        index += entry_cells;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn need_dtb_tests() {
        // TODO
    }
}
