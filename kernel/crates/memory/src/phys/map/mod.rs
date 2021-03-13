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

//! This module defines a platform-independent representation of a map of all of the system's
//! usable memory and a function for initializing the map that hides the platform-dependent details.

use {
    core::{
        cmp::{min, max, Ordering},
        ffi::c_void,
        mem::{self, size_of, align_of},
        num::NonZeroUsize
    },
    i18n::Text,
    crate::phys::ptr::PhysPtr,
    super::PHYS_ADDR_BITS
};

#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
mod dtb;

extern {
    static __start: c_void;
    static __end:   c_void;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub base: usize,
    pub size: usize,
    pub region_type: RegionType,
    pub hotpluggable: bool, // Indicates whether the memory region supports hotplugging.
    pub present: bool       // Indicates whether the memory is currently present in the system. Always true if not hotpluggable.
}

/// Defines the type of memory comprising a given region of the memory map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// Random-access memory
    Ram,
    /// Read-only memory
    Rom,
    /// Memory-mapped I/O
    Mmio
}

impl Region {
    // Compares two regions in a slightly unusual and asymmetric way that's useful for sorting
    // them. Specifically, this returns a tuple of two `Ordering`s, one for the address of the
    // first byte in the region represented by `self` and one for the address of the first byte
    // after that region. For each `Ordering`, the values are to be interpreted as follows:
    //  * `Ordering::Less`: That address is before `other_base`.
    //  * `Ordering::Equal`: That address is within the region represented by `other_base` and
    //      `other_size`.
    //  * `Ordering::Greater`: That address is after the region represented by `other_base` and
    //      `other_size`.
    pub(crate) fn compare_begin_end(&self, other_base: usize, other_size: usize) -> (Ordering, Ordering) {
        let start;
        let end;

        // Regions that include the top byte of the address space need special consideration.
        let self_contains_top = self.base.checked_add(self.size).is_none();
        let other_contains_top = other_base.checked_add(other_size).is_none();

        if self.base < other_base {
            start = Ordering::Less;
        } else if other_contains_top || self.base < other_base + other_size {
            start = Ordering::Equal;
        } else {
            start = Ordering::Greater;
        }

        if self_contains_top {
            if other_contains_top {
                end = Ordering::Equal;
            } else {
                end = Ordering::Greater;
            }
        } else if self.base + self.size <= other_base {
            end = Ordering::Less;
        } else if other_contains_top || self.base + self.size <= other_base + other_size {
            end = Ordering::Equal;
        } else {
            end = Ordering::Greater;
        }

        (start, end)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryMap {
    regions: [Option<Region>; Self::MAX_REGIONS],
    regions_count: usize
}

impl MemoryMap {
    const MAX_REGIONS: usize = 64;

    pub const fn new() -> MemoryMap {
        MemoryMap {
            regions: [None; Self::MAX_REGIONS],
            regions_count: 0
        }
    }

    /// Adds the given region of memory to the map.
    pub fn add_region(&mut self, base: usize, size: NonZeroUsize, region_type: RegionType, hotpluggable: bool) -> Result<(), ()> {
        let mut size = size.get();

        // Make sure the memory region uses only physical addresses that the architecture supports.
        // If it goes beyond that, crop it.
        let max_address = 1 << *PHYS_ADDR_BITS;
        if base >= max_address {
            return Ok(());
        }
        if base + size > max_address {
            size = max_address - base;
        }

        // Scan for the right place to add the region.
        let mut i = 0;
        while i < self.regions.len() {
            match self.regions[i] {
                Some(ref mut region) => { // This slot is taken. Decide what to do with it.
                    if region.hotpluggable || hotpluggable || region.region_type != region_type {
                        // For hotpluggable regions and regions with mismatching types, we allow
                        // overlaps.
                        if region.base > base {
                            // Case 1: Existing region is after the new region. Shift it to the right.
                            let new_region = Region {
                                base,
                                size,
                                region_type,
                                hotpluggable,
                                present: !hotpluggable
                            };
                            return self.insert(i, new_region);
                        }
                        // Case 2: Existing region is before the new region. Continue scanning.
                    } else if region.base > base + size {
                        // Case 1: Existing region is after the new region. Shift it to the right.
                        let new_region = Region {
                            base,
                            size,
                            region_type,
                            hotpluggable,
                            present: !hotpluggable
                        };
                        self.insert(i, new_region)?;
                        self.combine_right(i);
                        return Ok(());
                    } else if region.base + region.size >= base {
                        // Case 2: New region overlaps or is adjacent to existing region. Combine them.
                        let new_base = min(base, region.base);
                        let new_end = max(base + size, region.base + region.size);
                        region.base = new_base;
                        region.size = new_end - new_base;

                        self.combine_left(i);
                        self.combine_right(i);
                        return Ok(());
                    }
                    // Case 3: Existing region is before the new region. Continue scanning.
                },

                None => { // Empty slot found. Insert the region here.
                    return self.insert(i, Region {
                        base,
                        size,
                        region_type,
                        hotpluggable,
                        present: !hotpluggable
                    });
                }
            }

            i += 1;
        }

        // We ran out of room in the array.
        Err(())
    }

    /// Removes a region of memory from the map, whether or not it corresponds to an actual
    /// `Region` instance. If it returns `Err`, it's removed as many instances of that region as it
    /// could, but at least one remains.
    pub fn remove_region(&mut self, base: usize, size: NonZeroUsize) -> Result<(), ()> {
        let size = size.get();
        for i in (0 .. self.regions_count).rev() {
            if let Some(ref mut region) = self.regions[i] {
                // Pass 1: Handle every variation of overlap that doesn't require adding new
                // regions.
                match region.compare_begin_end(base, size) {
                    (Ordering::Less, Ordering::Less) => {},
                    (Ordering::Less, Ordering::Equal) => {
                        // [ Existing region ]
                        //          [ Piece being removed ]
                        region.size = base - region.base;
                    },
                    (Ordering::Less, Ordering::Greater) => {
                        // [           Existing region            ]
                        //         [ Piece being removed ]
                        // Nothing do do for the first pass.
                    },
                    (Ordering::Equal, Ordering::Less) => {
                        unsafe { unreachable_debug!(
                            "If the beginning of region 1 is inside region 2, the end of region 1 can't be before region 2."
                        ) }
                    },
                    (Ordering::Equal, Ordering::Equal) => {
                        //           [ Existing region ]
                        // [         Piece being removed          ]
                        self.remove(i);
                    },
                    (Ordering::Equal, Ordering::Greater) => {
                        //           [ Existing region ]
                        // [ Piece being removed ]
                        region.size = region.base.wrapping_add(region.size).wrapping_sub(base + size);
                        region.base = base + size;
                    },
                    (Ordering::Greater, Ordering::Less) => {
                        unsafe { unreachable_debug!(
                            "If the beginning of region 1 is after region 2, the end of region 1 can't be before region 2."
                        ) }
                    },
                    (Ordering::Greater, Ordering::Equal) => {
                        unsafe { unreachable_debug!(
                            "If the beginning of region 1 is after region 2, the end of region 1 can't be inside region 2."
                        ) }
                    },
                    (Ordering::Greater, Ordering::Greater) => {}
                };
            }
        }
        for i in (0 .. self.regions_count).rev() {
            if let Some(ref mut region) = self.regions[i] {
                // Pass 2: Handle the remaining variation, which requires splitting the affected
                // `Region`s.
                if region.compare_begin_end(base, size) == (Ordering::Less, Ordering::Greater) {
                    // [           Existing region            ]
                    //         [ Piece being removed ]
                    let part1_base = region.base;
                    let part1_size = base - region.base;
                    let part2_base = base + size;
                    let part2_size = region.base.wrapping_add(region.size).wrapping_sub(base + size);

                    let orig_base = region.base;
                    let orig_size = region.size;
                    region.base = part1_base;
                    region.size = part1_size;

                    let region_type = region.region_type;
                    let hotpluggable = region.hotpluggable;
                    mem::drop(region);

                    if let Err(()) = self.add_region(
                        part2_base,
                        unsafe { NonZeroUsize::new_unchecked(part2_size) },
                        region_type,
                        hotpluggable
                    ) {
                        // We couldn't make a new region. Return the existing region to its original size
                        // before returning so we won't remove more than was requested.
                        let region = self.regions[i].as_mut().unwrap();
                        region.base = orig_base;
                        region.size = orig_size;
                    }
                }
            }
        }
        Ok(())
    }

    /// A convenience method that returns an iterator over all the regions that are present.
    pub fn present_regions(&self) -> impl '_+Iterator<Item = Region> {
        self.regions.iter().filter_map(|opt| *opt).filter(|reg| reg.present)
    }

    // Inserts the given region at the given index.
    // Returns `Err(())` if there isn't enough room in the array.
    fn insert(&mut self, index: usize, region: Region) -> Result<(), ()> {
        if self.regions_count == self.regions.len() {
            return Err(());
        }

        let mut i = self.regions_count;
        while i > index {
            self.regions[i] = self.regions[i - 1];
            i -= 1;
        }
        self.regions[index] = Some(region);
        self.regions_count += 1;

        Ok(())
    }

    // Removes the region at the given index from the array.
    fn remove(&mut self, index: usize) {
        let mut i = index;
        while i < self.regions.len() - 1 && self.regions[i].is_some() {
            self.regions[i] = self.regions[i + 1];
            i += 1;
        }
        if i == self.regions.len() - 1 {
            self.regions[i] = None;
        }
        self.regions_count -= 1;
    }

    // Combines `self.regions[start_index]` with any regions to the left of it that can be combined.
    fn combine_left(&mut self, start_index: usize) {
        if start_index == 0 {
            return;
        }

        if let Some(ref left_region) = self.regions[start_index - 1] {
            if let Some(ref this_region) = self.regions[start_index] {
                let new_base = this_region.base;
                let new_size = this_region.size;

                if !left_region.hotpluggable && left_region.base + left_region.size >= new_base {
                    unsafe {
                        *(&left_region.size as *const _ as *mut _) = (new_base - left_region.base) + new_size;
                    }

                    self.remove(start_index);
                    self.combine_left(start_index - 1);
                }
            }
        }
    }

    // Combines `self.regions[start_index]` with any regions to the right of it that can be combined.
    fn combine_right(&mut self, start_index: usize) {
        if start_index + 1 >= self.regions.len() {
            return;
        }

        if let Some(ref right_region) = self.regions[start_index + 1] {
            if let Some(ref this_region) = self.regions[start_index] {
                if !right_region.hotpluggable &&
                        this_region.base + this_region.size >= right_region.base {
                    unsafe {
                        *(&this_region.size as *const _ as *mut _) = (right_region.base - this_region.base) + right_region.size;
                    }

                    self.remove(start_index + 1);
                    self.combine_right(start_index);
                }
            }
        }
    }
}

lazy_static! {
    unsafe {
        /// The system's memory map. All of the kernel's static memory is removed from the map as
        /// part of its initialization, so the heap doesn't need to be responsible for reserving
        /// that memory.
        pub static ref MEMORY_MAP: MemoryMap = {
            let mut map = build_memory_map()
                .expect(Text::memory_map_not_retrieved())
                .clone();

            let base = PhysPtr::<_, *const _>::from_virt(&__start as *const _).as_addr_phys();
            let size = PhysPtr::<_, *const _>::from_virt(&__end as *const _).as_addr_phys() - base;

            map.remove_region(base, NonZeroUsize::new(size).unwrap())
                .expect(Text::couldnt_reserve_kernel());
            map
        };
    }
}

// Builds a memory map using platform-specific means. 
fn build_memory_map() -> Result<&'static MemoryMap, ()> {
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))] {
        if let Some(ref parsed_dtb) = *dtb::PARSED_DTB {
            return Ok(&parsed_dtb.memory_map);
        }
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
        // TODO: Get the memory map using x86-specific methods.
        MemoryMap::new(); // This call is here only to remove the dead code warnings for now.
        unsafe {
            BOOT_INFO.a20_disabled(); // This call is here only to remove the dead code warnings for now.
        }
    }

    Err(())
}

extern {
    static BOOT_INFO: BootInfo;
}

// This struct contains all the information the bootloader provided us directly.
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
#[repr(C)]
struct BootInfo {
    size:  usize,         // The total size of this struct
    dtb:   *const c_void, // A pointer to the DTB, or null
}
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[repr(C)]
struct BootInfo {
    size:         usize,
    a20_disabled: bool    // If true, we'll need to remove every odd megabyte from the memory map.
}

impl BootInfo {
    /// Returns a pointer to the DTB, or null if there is none.
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    pub fn dtb(&self) -> *const c_void {
        self.validate();
        self.dtb
    }

    /// Returns `true` if the A20 line has been disabled, else `false`.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn a20_disabled(&self) -> bool {
        self.validate();
        self.a20_disabled
    }

    // Validates the structure and panics if it's invalid. This should be called before any field
    // of the structure is used.
    fn validate(&self) {
        assert_eq!(self.size as usize, size_of::<BootInfo>());
        assert!(self as *const _ as usize % align_of::<BootInfo>() == 0);
    }
}

#[cfg(test)]
mod tests {
    use core::mem;
    use core::num::NonZeroUsize;
    use super::{MemoryMap, Region, RegionType};

    #[test]
    fn empty_memory_map() {
        let map = MemoryMap::new();
        println!("map = {:?}", map);
        assert_eq!(map.present_regions().next(), None);
    }

    #[test]
    fn too_many_regions() {
        let mut map = MemoryMap::new();
        let mut i = 0;
        while i < 64 {
            if map.add_region(i * 2, NonZeroUsize::new(1).unwrap(), RegionType::Ram, false).is_err() {
                panic!("memory map ran out of room after only {} regions were added", i);
            }
            i += 1;
        }
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        let mut j = 0;
        while j < 64 {
            assert_eq!(regions.next(), Some(Region { base: j * 2, size: 1, hotpluggable: false, present: true, region_type: RegionType::Ram }));
            j += 1;
        }
        assert_eq!(regions.next(), None);
        mem::drop(regions);

        if map.add_region(i * 2, NonZeroUsize::new(1).unwrap(), RegionType::Ram, false).is_ok() {
            panic!("memory map did not return an error after {} regions were added; map = {:?}", i, map);
        }
    }

    #[test]
    fn one_region() {
        let mut map = MemoryMap::new();
        map.add_region(0x4000_0000, NonZeroUsize::new(0x3f_c000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x4000_0000, size: 0x3f_c000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn one_region_hotpluggable() {
        let mut map = MemoryMap::new();
        map.add_region(0x4000_0000, NonZeroUsize::new(0x3f_c000_0000).unwrap(), RegionType::Ram, true).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        //assert_eq!(regions.next(), Some(Region { base: 0x4000_0000, size: 0x3f_c000_0000, hotpluggable: true, present: false, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn two_regions() {
        let mut map = MemoryMap::new();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x8000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), Some(Region { base: 0x1_0000_0000, size: 0x1_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn two_regions_reversed() {
        let mut map = MemoryMap::new();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x8000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), Some(Region { base: 0x1_0000_0000, size: 0x1_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_123() {
        let mut map = MemoryMap::new();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x2_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_132() {
        let mut map = MemoryMap::new();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x2_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_213() {
        let mut map = MemoryMap::new();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x2_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_231() {
        let mut map = MemoryMap::new();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x2_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_312() {
        let mut map = MemoryMap::new();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x2_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_321() {
        let mut map = MemoryMap::new();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x2_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_123_hotpluggable() {
        let mut map = MemoryMap::new();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, true).unwrap();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x8000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        //assert_eq!(regions.next(), Some(Region { base: 0x8000_0000, size: 0x8000_0000, hotpluggable: true, present: false, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), Some(Region { base: 0x1_0000_0000, size: 0x1_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn three_regions_adjacent_321_hotpluggable() {
        let mut map = MemoryMap::new();
        map.add_region(0x1_0000_0000, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, true).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x8000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x8000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        //assert_eq!(regions.next(), Some(Region { base: 0x8000_0000, size: 0x8000_0000, hotpluggable: true, present: false, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), Some(Region { base: 0x1_0000_0000, size: 0x1_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn superset_subset() {
        let mut map = MemoryMap::new();
        map.add_region(0x0, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x1000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x1_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn subset_superset() {
        let mut map = MemoryMap::new();
        map.add_region(0x8000_0000, NonZeroUsize::new(0x1000_0000).unwrap(), RegionType::Ram, false).unwrap();
        map.add_region(0x0, NonZeroUsize::new(0x1_0000_0000).unwrap(), RegionType::Ram, false).unwrap();
        println!("map = {:?}", map);

        let mut regions = map.present_regions();
        assert_eq!(regions.next(), Some(Region { base: 0x0, size: 0x1_0000_0000, hotpluggable: false, present: true, region_type: RegionType::Ram }));
        assert_eq!(regions.next(), None);
    }

    #[test]
    fn need_non_ram_regions() {
        // TODO
    }

    #[test]
    fn need_region_removal_tests() {
        // TODO
    }
}
