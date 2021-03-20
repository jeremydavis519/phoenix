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

//! This module defines the paging structure for Aarch64 targets.

use {
    alloc::{
        alloc::AllocError,
        vec::Vec
    },
    core::{
        convert::{TryFrom, TryInto},
        fmt,
        ffi::c_void,
        mem::{self, size_of},
        num::NonZeroUsize,
        ptr,
        sync::atomic::{AtomicU8, AtomicUsize, AtomicPtr, Ordering}
    },
    hashbrown::HashSet, // FIXME: Implement our own `HashSet`.
    i18n::Text,
    shared::{
        count_cpus,
        cpu_index,
        sync::atomic::Atomic64Bit
    },
    crate::{
        allocator::AllMemAlloc,
        phys::{
            RegionType,
            block::{Block, BlockMut},
            map::MEMORY_MAP,
            ptr::PhysPtr
        }
    }
};

// The number of bytes per translation granule (i.e. per smallest page)
static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

const KERNEL_ASID: u16 = 0;

/// Returns the size of a page of virtual memory.
pub fn page_size() -> usize {
    PAGE_SIZE.load(Ordering::Acquire)
}

// The maximum number of bits that can be used in a physical address (probably 48 or 52)
static MAX_PHYS_BITS: AtomicU8 = AtomicU8::new(0);

// The number of bits that can be used in a virtual address (probably 48 or 52)
static MAX_VIRT_BITS: AtomicU8 = AtomicU8::new(0);

// The values to plug into `PageEntry::ATTR_INDEX` for different kinds of memory
// Also defined in /src/arch/arch64/paging.S (make sure to keep them up to date)
const ATTR_NORMAL_MEMORY: u8 = 0;
const ATTR_DEVICE_MEMORY: u8 = 1;

extern {
    // Markers for the beginnings and ends of relevant parts of the kernel
    static __readonly_start: c_void;
    static __readonly_end: c_void;
    static __rw_shareable_start: c_void;
    static __rw_shareable_end: c_void;
    static __rw_nonshareable_start: c_void;
    static __rw_nonshareable_end: c_void;
    static __trampoline_start: c_void;
    static __trampoline_ro_start: c_void;
    static __trampoline_ro_end: c_void;
    static __trampoline_rw_start: c_void;
    static __trampoline_rw_end: c_void;
    static __trampoline_end: c_void;

    // The virtual base address of the trampoline, the link between kernelspace and userspace (and the
    // only part of the kernel that should always be mapped to virtual memory on any system susceptible
    // to Meltdown)
    static __trampoline_virt: c_void;

    static __trampoline_stacks_virt: c_void;
}

fn readonly_start() -> usize  { unsafe { &__readonly_start as *const _ as usize } }
fn readonly_end() -> usize    { unsafe { &__readonly_end as *const _ as usize } }
fn rw_shareable_start() -> usize { unsafe { &__rw_shareable_start as *const _ as usize } }
fn rw_shareable_end() -> usize   { unsafe { &__rw_shareable_end as *const _ as usize } }
fn rw_nonshareable_start() -> usize { unsafe { &__rw_nonshareable_start as *const _ as usize } }
fn rw_nonshareable_end() -> usize   { unsafe { &__rw_nonshareable_end as *const _ as usize } }
fn trampoline_start() -> usize { unsafe { &__trampoline_start as *const _ as usize } }
fn trampoline_ro_start() -> usize { unsafe { &__trampoline_ro_start as *const _ as usize } }
fn trampoline_ro_end() -> usize   { unsafe { &__trampoline_ro_end as *const _ as usize } }
fn trampoline_rw_start() -> usize { unsafe { &__trampoline_rw_start as *const _ as usize } }
fn trampoline_rw_end() -> usize   { unsafe { &__trampoline_rw_end as *const _ as usize } }
fn trampoline_end() -> usize   { unsafe { &__trampoline_end as *const _ as usize } }
fn trampoline_virt() -> usize { unsafe { &__trampoline_virt as *const _ as usize } }
fn trampoline_stacks_virt() -> usize { unsafe { &__trampoline_stacks_virt as *const _ as usize } }

const TRAMPOLINE_STACK_SIZE: usize = 1024;

/// Converts the given pointer into one that points into the trampoline code (the code that is
/// mapped in all address spaces). Panics if the given pointer isn't in the trampoline.
pub fn trampoline<T>(ptr: *const T) -> *const T {
    assert!((ptr as usize) >= trampoline_start(), "pointer {:p} is before the trampoline ({:#x} to {:#x})",
        ptr, trampoline_start(), trampoline_end());
    assert!((ptr as usize) < trampoline_end(), "pointer {:p} is after the trampoline ({:#x} to {:#x})",
        ptr, trampoline_start(), trampoline_end());
    (ptr as usize - trampoline_start() + trampoline_virt()) as *const T
}

/// Returns the stack pointer for this CPU when it's in the trampoline and its stack is empty.
pub fn trampoline_stack_ptr() -> usize {
    trampoline_stacks_virt() + (cpu_index() + 1) * TRAMPOLINE_STACK_SIZE
}

pub(crate) static ROOT_PAGE_TABLE: AtomicPtr<RootPageTable> = AtomicPtr::new(ptr::null_mut());
static TRAMPOLINE_PAGE_TABLE: AtomicPtr<RootPageTable> = AtomicPtr::new(ptr::null_mut());

lazy_static! {
    unsafe {
        // A page filled with zeroes, to be used with copy-on-write semantics for uninitialized
        // data.
        static ref ZEROES_PAGE: Block<u8> = {
            let page_size = page_size();
            // This block will always be physically contiguous, even if we stop identity-mapping the
            // kernel, because it's confined to a single page.
            let block_mut = AllMemAlloc.malloc::<u8>(
                page_size,
                NonZeroUsize::new(page_size).expect("tried to allocate the zeroes page before the page size is known")
            ).expect("failed to allocate the zeroes page");

            for i in 0 .. block_mut.size() {
                *block_mut.index(i) = 0;
            }

            Block::from(block_mut)
        };
    }
}

ffi_enum! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy)]
    /// Specifies the exception level that will use a given page table.
    pub enum ExceptionLevel {
        /// Accessible at EL0 and above
        El0 = 0,
        /// Accessible at EL1 and above
        El1 = 1
    }
}

/// The basic status of a page.
#[derive(Debug)]
pub enum PageStatus {
    /// The page is mapped (although it might be in a swapfile).
    Mapped,
    /// The page is unmapped, but only temporarily. A CPU is working on remapping it.
    TempUnmapped,
    /// The page is unmapped, and no CPU is working on it.
    Unmapped
}

// Specifies the set of other observers on the system with which the hardware needs to keep a page
// consistent. This can be set for each page individually.
#[derive(Debug, Clone, Copy)]
enum ShareabilityDomain {
    // No need for consistency because only one observer (e.g. CPU core) will ever see this page.
    // For instance, each core's private stack in the kernel's address space can be non-shareable if
    // it occupies a whole page. This is the most dangerous shareability domain, but also the
    // fastest. As an example of the danger, the trampoline stacks cannot be non-shareable unless
    // they are kept from overlapping the same cache lines, since flushing a cache line could then
    // destroy another core's stack.
    NonShareable,
    // Shareable with other observers in a tight cluster (implementation-defined, but could be a
    // CPU cluster, for instance). Since this is implementation-defined, we should probably avoid it
    // in general.
    #[allow(dead_code)]
    Inner,
    // Shareable with multiple inner shareable domains. This is the safest shareability domain, but
    // also the slowest.
    Outer
}

// This macro exists to reduce the code duplication that was arising from supporting three different
// translation granule sizes.
macro_rules! define_root_page_table {
    ($(
        $table:ident {
            $table_root:ident;
            $page_size:expr
        }
    ),*) => {
        /// The root page table of an address space. When this is dropped, the entire address space
        /// is unmapped.
        #[derive(Debug)]
        pub struct RootPageTable {
            internals: RootPageTableInternal,
            exception_level: ExceptionLevel,
            asid: u16
        }

        #[derive(Debug)]
        enum RootPageTableInternal {
            $(
                #[doc(hidden)]
                $table(BlockMut<$table_root>)
            ),*
        }

        impl RootPageTable {
            // Constructs a new, empty root page table.
            fn new(exception_level: ExceptionLevel, asid: u16) -> Result<BlockMut<RootPageTable>, AllocError> {
                let block = AllMemAlloc.malloc::<RootPageTable>(
                    mem::size_of::<RootPageTable>(),
                    NonZeroUsize::new(mem::align_of::<RootPageTable>()).unwrap()
                )?;
                let root = unsafe { &mut *block.index(0) };
                mem::forget(mem::replace(&mut root.exception_level, exception_level));
                mem::forget(mem::replace(&mut root.asid, asid));
                match page_size() {
                    $(
                        $page_size => {
                            let table_block = AllMemAlloc.malloc::<$table_root>(
                                mem::size_of::<$table_root>(),
                                NonZeroUsize::new(mem::align_of::<$table_root>()).unwrap()
                            )?;
                            unsafe {
                                <$table_root>::make_new(table_block.index(0));
                            }
                            mem::forget(mem::replace(&mut root.internals, RootPageTableInternal::$table(table_block)));
                        },
                    )*
                    page_size => panic!("unsupported page size {:#x}", page_size)
                };
                Ok(block)
            }

            /// Constructs a new, empty root page table for a userspace application.
            pub fn new_userspace(asid: u16) -> Result<BlockMut<RootPageTable>, AllocError> {
                RootPageTable::new(ExceptionLevel::El0, asid)
            }

            /// Returns the pointer to the root page table from the processor's perspective. This isn't
            /// type-safe, though, so it shouldn't be used in Rust. But it can be stored in TTBRx_EL1.
            pub fn table_ptr(&self) -> *const c_void {
                match self.internals {
                    $(
                        RootPageTableInternal::$table(ref table_block) =>  {
                            table_block.index(0) as *const c_void
                        }
                    ),*
                }
            }

            // Identity-maps the given list of regions. The given bases and sizes are required to be
            // multiples of `page_size()`. Any byte that is listed in two or more regions is included in
            // the first of those regions and excluded from the rest.
            fn identity_map(regions: &[(usize, NonZeroUsize, RegionType, ShareabilityDomain)], asid: u16)
                    -> Result<BlockMut<RootPageTable>, AllocError> {
                let page_size = page_size();

                assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);

                let block = AllMemAlloc.malloc::<RootPageTable>(
                    mem::size_of::<RootPageTable>(),
                    NonZeroUsize::new(mem::align_of::<RootPageTable>()).unwrap()
                )?;
                let root = unsafe { &mut *block.index(0) };
                root.exception_level = ExceptionLevel::El1;
                root.asid = asid;

                match page_size {
                    $(
                        $page_size => {
                            let table_block = AllMemAlloc.malloc::<$table_root>(
                                mem::size_of::<$table_root>(),
                                NonZeroUsize::new(mem::align_of::<$table_root>()).unwrap()
                            )?;
                            unsafe {
                                <$table_root>::identity_map(table_block.index(0), regions)?;
                            }
                            root.internals = RootPageTableInternal::$table(table_block);
                        },
                    )*
                    _ => panic!("unsupported page size {:#x}", page_size)
                };
                Ok(block)
            }

            /// Maps the given region to the given virtual base address.
            ///
            /// If any of these pages have already been mapped, the mapping fails without having mapped
            /// any of them.
            ///
            /// The given bases and size are required to be multiples of `page_size()`.
            ///
            /// Setting `virt_base` to `None` causes a virtual base address to be chosen automatically
            /// such that the mapped region doesn't overlap any region that is already mapped.
            ///
            /// # Returns
            /// `Ok(addr)` on success, where `addr` is the base address of the mapped region, else
            /// `Err(())`.
            pub fn map(&self, phys_base: usize, virt_base: Option<usize>, size: NonZeroUsize, reg_type: RegionType)
                    -> Result<usize, ()> {
                self.map_impl(
                    phys_base,
                    virt_base,
                    size,
                    Some(reg_type),
                    PageEntry::NOT_DIRTY | PageEntry::ONE,
                    PageEntry::UNMAPPED
                ).map_err(|_| ())
            }

            // Like `map` but marks the pages as already dirty. This is used to prevent non-identity-
            // mapped kernel pages from causing infinite Permission Faults.
            fn map_dirty(&self, phys_base: usize, virt_base: Option<usize>, size: NonZeroUsize, reg_type: RegionType)
                    -> Result<usize, ()> {
                self.map_impl(
                    phys_base,
                    virt_base,
                    size,
                    Some(reg_type),
                    PageEntry::ONE,
                    PageEntry::UNMAPPED
                ).map_err(|_| ())
            }

            /// Maps the given region to the given virtual base address, marking it for Copy-on-Write.
            ///
            /// If any of these pages have already been mapped, the mapping fails without having mapped
            /// any of them.
            ///
            /// The given bases and size are required to be multiples of `page_size()`.
            ///
            /// Setting `virt_base` to `None` causes a virtual base address to be chosen automatically
            /// such that the mapped region doesn't overlap any region that is already mapped.
            ///
            /// # Returns
            /// `Ok(addr)` on success, where `addr` is the base address of the mapped region, else
            /// `Err(())`.
            pub fn map_cow(&self, phys_base: usize, virt_base: Option<usize>, size: NonZeroUsize)
                    -> Result<usize, ()> {
                self.map_impl(
                    phys_base,
                    virt_base,
                    size,
                    Some(RegionType::Ram),
                    PageEntry::NOT_DIRTY | PageEntry::COW | PageEntry::ONE,
                    PageEntry::UNMAPPED
                ).map_err(|_| ())
            }

            /// Maps a zero-filled page to the given virtual addresses, marking them as copy-on-write.
            ///
            /// If any of these pages have already been mapped, the mapping fails without having mapped
            /// any of them.
            ///
            /// The given base and size are required to be multiples of `page_size()`.
            ///
            /// Setting `virt_base` to `None` causes a virtual base address to be chosen automatically
            /// such that the mapped region doesn't overlap any region that is already mapped.
            ///
            /// # Returns
            /// `Ok(addr)` on success, where `addr` is the base address of the mapped region, else
            /// `Err(())`.
            pub fn map_zeroed(&self, virt_base: Option<usize>, size: NonZeroUsize)
                    -> Result<usize, ()> {
                self.map_zeroed_impl(virt_base, size, PageEntry::UNMAPPED)
                    .map_err(|_| ())
            }

            /// Marks the given virtual addresses as being stored in the process's executable file.
            ///
            /// If any of these pages have already been mapped, the mapping fails without having mapped
            /// any of them.
            ///
            /// The given base and size are required to be multiples of `page_size()`.
            ///
            /// Setting `virt_base` to `None` causes a virtual base address to be chosen automatically
            /// such that the mapped region doesn't overlap any region that is already mapped.
            ///
            /// # Returns
            /// `Ok(addr)` on success, where `addr` is the base address of the mapped region, else
            /// `Err(())`.
            pub fn map_exe_file(&self, virt_base: Option<usize>, size: NonZeroUsize)
                    -> Result<usize, ()> {
                self.map_impl(
                    0,
                    virt_base,
                    size,
                    None,
                    PageEntry::IN_EXE_FILE,
                    PageEntry::UNMAPPED
                ).map_err(|_| ())
            }

            /// Maps the given physical addresses to the given virtual addresses that were previously
            /// marked as being stored in the process's executable file.
            ///
            /// If any of these pages are not marked as being in the executable file, the mapping fails
            /// without having mapped any of them.
            ///
            /// The given base and size are required to be multiples of `page_size()`.
            ///
            /// # Returns
            /// `Ok(())` on success, else `Err(())`.
            pub fn map_from_exe_file(
                    &self,
                    phys_base: usize,
                    virt_base: usize,
                    size: NonZeroUsize,
                    reg_type: RegionType
            ) -> Result<(), ()> {
                self.map_impl(
                    phys_base,
                    Some(virt_base),
                    size,
                    Some(reg_type),
                    PageEntry::NOT_DIRTY | PageEntry::ONE,
                    PageEntry::IN_EXE_FILE
                )
                    .map(|_| ())
                    .map_err(|_| ())
            }

            /// Maps a zero-filled page to the given virtual addresses, marking them as copy-on-write.
            ///
            /// If any of these pages are not marked as being in the executable file, the mapping fails
            /// without having mapped any of them.
            ///
            /// The given base and size are required to be multiples of `page_size()`.
            ///
            /// # Returns
            /// `Ok(())` on success, else `Err(())`.
            pub fn map_zeroed_from_exe_file(&self, virt_base: usize, size: NonZeroUsize) -> Result<(), ()> {
                self.map_zeroed_impl(Some(virt_base), size, PageEntry::IN_EXE_FILE)
                    .map(|_| ())
                    .map_err(|_| ())
            }

            fn map_impl(
                    &self,
                    phys_base: usize,
                    virt_base: Option<usize>,
                    size: NonZeroUsize,
                    reg_type: Option<RegionType>,
                    mut page_flags: PageEntry,
                    expected: PageEntry
            ) -> Result<usize, Option<NonZeroUsize>> {
                let page_size = page_size();

                assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);
                assert!(phys_base.checked_add(size.get() - 1).is_some(),
                    "physical address range overflows integer size (base: {:#018x}, size: {:#018x})",
                    phys_base, size.get());
                assert!(phys_base + (size.get() - 1) < 1 << usize::from(MAX_PHYS_BITS.load(Ordering::Acquire)),
                    "physical address out of range: {:#018x}", phys_base + (size.get() - 1));

                match virt_base {
                    Some(virt_base) => {
                        // We know which base address to use.
                        assert!(virt_base.checked_add(size.get() - 1).is_some(),
                            "virtual address range overflows integer size (base: {:#018x}, size: {:#018x})",
                            virt_base, size.get());
                        assert!(virt_base + (size.get() - 1) < 1 << usize::from(MAX_VIRT_BITS.load(Ordering::Acquire)),
                            "virtual address out of range: {:#018x}", virt_base + (size.get() - 1));

                        // If the page is going to be fully mapped (i.e. so the MMU can use it), it needs a bunch of
                        // extra flags.
                        if page_flags.contains(PageEntry::ONE) {
                            page_flags |= match reg_type {
                                Some(RegionType::Ram) => PageEntry::normal_memory() | PageEntry::UXN | PageEntry::PXN
                                    | if page_flags.contains(PageEntry::COW) { PageEntry::empty() } else { PageEntry::DBM },
                                Some(RegionType::Rom) => PageEntry::normal_memory(),
                                Some(RegionType::Mmio) => PageEntry::device_memory(),
                                None => PageEntry::empty()
                            } | match self.exception_level {
                                ExceptionLevel::El0 => PageEntry::PXN | PageEntry::EL0,
                                ExceptionLevel::El1 => PageEntry::UXN | PageEntry::ACCESSED
                            } | PageEntry::SHAREABLE;
                        }

                        match self.internals {
                            $(
                                RootPageTableInternal::$table(ref table_block) => {
                                    let table = unsafe { &mut *table_block.index(0) };
                                    table.map(
                                        phys_base,
                                        virt_base,
                                        size,
                                        self.exception_level,
                                        self.asid,
                                        page_flags,
                                        expected
                                    )?;
                                    Ok(virt_base)
                                }
                            ),*
                        }
                    },
                    None => {
                        // We have to determine a range of virtual addresses that will work. We use a
                        // first-fit allocator, trying to map everywhere until we find a region that works.
                        let mut virt_base = page_size; // Avoid allocating the zero page.
                        loop {
                            match self.map_impl(
                                    phys_base,
                                    Some(virt_base),
                                    size,
                                    reg_type,
                                    page_flags,
                                    expected
                            ) {
                                Ok(addr) => return Ok(addr),
                                Err(Some(next_addr)) => virt_base = next_addr.get(),
                                Err(None) => return Err(None)
                            };
                        }
                    }
                }
            }

            fn map_zeroed_impl(&self, virt_base: Option<usize>, size: NonZeroUsize, expected: PageEntry)
                    -> Result<usize, Option<NonZeroUsize>> {
                let page_size = page_size();
                assert_eq!(ZEROES_PAGE.size(), page_size);
                assert_eq!(size.get() % page_size, 0);

                match virt_base {
                    Some(virt_base) => {
                        // We know which base address to use.
                        assert_eq!(virt_base % page_size, 0);

                        let mut addr_virt = virt_base;
                        let end = virt_base.wrapping_add(size.get());
                        while addr_virt != end {
                            match self.map_impl(
                                    PhysPtr::<u8, *const u8>::from_virt(ZEROES_PAGE.index(0)).as_addr_phys(),
                                    Some(addr_virt),
                                    NonZeroUsize::new(page_size).unwrap(),
                                    Some(RegionType::Ram),
                                    PageEntry::NOT_DIRTY | PageEntry::COW | PageEntry::ONE,
                                    expected
                                ) {
                                Ok(_) => {},
                                Err(next_addr) => {
                                    // The mapping failed. Undo what we've done so far.
                                    if let Some(size) = NonZeroUsize::new(addr_virt.wrapping_sub(virt_base)) {
                                        self.unmap_impl(virt_base, size, expected);
                                    }
                                    return Err(next_addr);
                                }
                            };
                            addr_virt = addr_virt.wrapping_add(page_size);
                        }
                        Ok(virt_base)
                    },
                    None => {
                        // We have to determine a range of virtual addresses that will work. We use a
                        // first-fit allocator, trying to map everywhere until we find a region that works.
                        let mut virt_base = page_size; // Avoid allocating the zero page.
                        loop {
                            match self.map_zeroed_impl(Some(virt_base), size, expected) {
                                Ok(addr) => return Ok(addr),
                                Err(Some(next_addr)) => virt_base = next_addr.get(),
                                Err(None) => return Err(None)
                            };
                        }
                    }
                }
            }

            /// Unmaps all the pages in the given range.
            ///
            /// The given base and size are required to be multiples of `page_size()`.
            pub fn unmap(&self, virt_base: usize, size: NonZeroUsize) {
                self.unmap_impl(virt_base, size, PageEntry::UNMAPPED)
            }

            fn unmap_impl(&self, virt_base: usize, size: NonZeroUsize, new_entry: PageEntry) {
                match self.internals {
                    $(
                        RootPageTableInternal::$table(ref table_block) => {
                            let table = unsafe { &*table_block.index(0) };
                            table.unmap(virt_base, size, self.asid, new_entry)
                        }
                    ),*
                }
            }

            /// Returns the current status of the page containing the given address.
            pub fn page_status(&self, virt_base: usize) -> PageStatus {
                assert!(virt_base < 1 << usize::from(MAX_VIRT_BITS.load(Ordering::Acquire)),
                    "virtual address out of range: {:#018x}", virt_base);

                match self.internals {
                    $(
                        RootPageTableInternal::$table(ref table_block) => {
                            let table = unsafe { &*table_block.index(0) };
                            table.page_status(virt_base)
                        }
                    ),*
                }
            }

            /// Determines whether the page at the given virtual address is currently in a swapfile and,
            /// if so, returns its location in that swapfile.
            pub fn location_in_swapfile(&self, virt_addr: usize) -> Option<u64> {
                match self.internals {
                    $(
                        RootPageTableInternal::$table(ref table_block) => {
                            let table = unsafe { &*table_block.index(0) };
                            table.location_in_swapfile(virt_addr)
                        }
                    ),*
                }
            }
        }

        unsafe impl Sync for RootPageTable {}
    };
}

define_root_page_table! {
    Table4k {
        Level0PageTable4k;
        0x1000
    },
    Table16k {
        Level0PageTable16k;
        0x4000
    },
    Table64k {
        Level1PageTable64k;
        0x10000
    }
}

macro_rules! define_page_table {
    ($table:ident: $entry_type:ty, $entries_count:expr; $align:expr) => {
        #[doc(hidden)]
        #[repr(C, align($align))]
        #[derive(Debug)]
        pub struct $table {
            entries: [Atomic64Bit<$entry_type>; $entries_count]
        }
    };
}

macro_rules! impl_branch_table {
    ( $table:ty, $next:ty : bits $bits_lo:expr => $bits_hi:expr ; kiB $granule_size:expr ) => {
        impl_branch_table!(@internal $table, $next: bits $bits_lo => $bits_hi; kiB $granule_size; true);
    };
    ( $table:ty, $next:ty : bits $bits_lo:expr => $bits_hi:expr ; kiB $granule_size:expr ; tables only ) => {
        impl_branch_table!(@internal $table, $next: bits $bits_lo => $bits_hi; kiB $granule_size; false);
    };

    ( @internal $table:ty, $next:ty : bits $bits_lo:expr => $bits_hi:expr ; kiB $granule_size:expr ; $blocks_allowed:expr ) => {
        impl $table {
            unsafe fn make_new(table: *mut $table) {
                for entry in (*table).entries.iter() {
                    entry.store(Descriptor { table: PageTableEntry::UNMAPPED }, Ordering::Release);
                }
            }

            // Calculates the amount of space that would be required for translation tables at the
            // level of this type of table and all their subtables in order to identity-map the
            // given regions.
            #[allow(dead_code)] // This function is never called on the root tables.
            fn identity_map_size(regions: &[(usize, NonZeroUsize, RegionType, ShareabilityDomain)]) -> usize {
                let block_size = 1 << $bits_lo;
                let bigger_block_size = 2 << $bits_hi;

                let mut breaks = HashSet::new();
                let mut bigger_block_bases = HashSet::new();
                let mut total_size = 0;
                for &(base, size, ty, shareability) in regions {
                    // Space for the pages at this level
                    let mut bigger_block_base = base & !(bigger_block_size - 1);
                    while bigger_block_base < base + size.get() {
                        if bigger_block_bases.insert(bigger_block_base) {
                            total_size += size_of::<$table>();
                        }
                        bigger_block_base += bigger_block_size;
                    }

                    // Space for the pages at the next level
                    if $blocks_allowed {
                        // Since blocks are allowed, the only increases to the size are for
                        // misaligned pages at the beginning and end.
                        if base % block_size != 0 && breaks.insert(base / block_size) {
                            if let Some(size) = NonZeroUsize::new(block_size - (base % block_size)) {
                                total_size += <$next>::identity_map_size(&[(base, size, ty, shareability)]);
                            }
                        }
                        let end = base + size.get();
                        if end % block_size != 0 && breaks.insert(end / block_size) {
                            if let Some(size) = NonZeroUsize::new(end % block_size) {
                                total_size += <$next>::identity_map_size(&[(end - size.get(), size, ty, shareability)]);
                            }
                        }
                    } else {
                        total_size += <$next>::identity_map_size(regions);
                    }
                }

                total_size
            }

            // Identity-maps the given ranges of addresses. If two regions in the list contain the
            // same page, it is mapped only once, for the first region. The given bases and sizes
            // are required to be multiples of `PAGE_SIZE`.
            #[allow(dead_code)] // This function is only called on the root tables.
            unsafe fn identity_map(table: *mut $table, regions: &[(usize, NonZeroUsize, RegionType, ShareabilityDomain)]) -> Result<(), AllocError> {
                // This block actually consists of all of the lower-level page tables, not just
                // those in the next level. Its correctness relies on the fact that all of the non-
                // root translation tables always have the same size as each other and they're all
                // laid out in the same way.
                let subtables = AllMemAlloc.malloc::<$next>(<$next>::identity_map_size(regions), NonZeroUsize::new(mem::align_of::<$next>()).unwrap())?;
                for i in 0 .. subtables.size() {
                    <$next>::make_new(subtables.index(i) as *mut $next);
                }

                let mut subtable_addr = subtables.index(0) as usize;
                for &(base, size, region_type, shareability) in regions {
                    subtable_addr = Self::identity_map_single_region(&mut *table, base, size, region_type, shareability, subtable_addr);
                }
                mem::forget(subtables);

                Ok(())
            }

            // Identity-maps the given region, with one region type, and returns the address of the
            // next translation table that should be added if there is more to map.
            fn identity_map_single_region(dest: &mut $table, base: usize, size: NonZeroUsize,
                    region_type: RegionType, shareability: ShareabilityDomain, mut subtable_addr: usize) -> usize {
                let block_size = 1 << $bits_lo;
                let page_size = page_size();
                let bigger_block_size = 2 << $bits_hi;
                let size = size.get();
                let bigger_base = base & !(bigger_block_size - 1);

                assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);
                assert_eq!(base % page_size, 0, "{}", Text::PagesBaseMisaligned(base));
                assert_eq!(size % page_size, 0, "{}", Text::PagesSizeMisaligned(size));

                let mut addr = base;
                let end = base + size;
                while addr < end {
                    let index = (addr - bigger_base) / block_size;
                    if $blocks_allowed && addr % block_size == 0 && addr + block_size <= end {
                        // We can map a whole large page here.
                        let type_flags = match region_type {
                            RegionType::Ram => PageEntry::normal_memory() | PageEntry::UXN | PageEntry::PXN,
                            RegionType::Rom => PageEntry::NOT_DIRTY | PageEntry::normal_memory(), // NOT_DIRTY = read-only in this context
                            RegionType::Mmio => PageEntry::device_memory()
                        };
                        let shareability_flags = match shareability {
                            ShareabilityDomain::NonShareable => PageEntry::empty(),
                            ShareabilityDomain::Inner => PageEntry::SHAREABLE | PageEntry::INNER,
                            ShareabilityDomain::Outer => PageEntry::SHAREABLE
                        };
                        let new_descriptor = Descriptor {
                            page: PageEntry::UXN
                                | PageEntry::from_address(addr as u64).unwrap()
                                | PageEntry::ACCESSED
                                | shareability_flags
                                | type_flags
                                | PageEntry::ONE
                        };
                        match (*dest).entries[index].compare_exchange(Descriptor { table: PageTableEntry::UNMAPPED },
                                new_descriptor, Ordering::AcqRel, Ordering::Acquire) {
                            Ok(_) => {}, // Success!
                            Err(desc) => {
                                unsafe {
                                    if desc.table.contains(PageTableEntry::ONE) {
                                        // At least one page has already been mapped in this block. To
                                        // avoid overwriting it, map pages in smaller chunks.
                                        let subtable = &mut *(desc.table.address($granule_size) as *mut $next);
                                        subtable_addr = <$next>::identity_map_single_region(
                                            subtable, addr, NonZeroUsize::new(block_size).unwrap(), region_type, shareability, subtable_addr
                                        );
                                    } else {
                                        // The entire block has already been mapped. To avoid
                                        // overwriting it, do nothing.
                                    }
                                }
                            }
                        };
                    } else {
                        // This piece doesn't cover a whole block of size block_size, or block
                        // descriptors aren't allowed at this translation level. Delegate the task
                        // to a subtable.
                        let new_descriptor = Descriptor {
                            table: PageTableEntry::UXN
                                | PageTableEntry::from_address(subtable_addr as u64).unwrap()
                                | PageTableEntry::ONE
                        };
                        match (*dest).entries[index].compare_exchange(Descriptor { table: PageTableEntry::UNMAPPED },
                                new_descriptor, Ordering::AcqRel, Ordering::Acquire) {
                            Ok(_) => {
                                unsafe {
                                    let subtable = &mut *(new_descriptor.table.address($granule_size) as *mut $next);
                                    subtable_addr += mem::size_of::<$next>();
                                    let subsize = NonZeroUsize::new(usize::min(end - addr, block_size - (addr % block_size))).unwrap();
                                    subtable_addr = <$next>::identity_map_single_region(
                                        subtable, addr, subsize, region_type, shareability, subtable_addr
                                    );
                                }
                            },
                            Err(desc) => {
                                unsafe {
                                    if desc.table.contains(PageTableEntry::ONE) {
                                        let subtable = &mut *(desc.table.address($granule_size) as *mut $next);
                                        let subsize = NonZeroUsize::new(usize::min(end - addr, block_size - (addr % block_size))).unwrap();
                                        subtable_addr = <$next>::identity_map_single_region(
                                            subtable, addr, subsize, region_type, shareability, subtable_addr
                                        );
                                    } else {
                                        // This was already mapped as part of a block. We're not
                                        // replacing mapped pages, so do nothing.
                                    }
                                }
                            }
                        };
                    }
                    addr += block_size;
                }

                subtable_addr
            }

            // Maps the given region to the given virtual base address.
            //
            // If any of these pages have already been mapped, the mapping fails without having
            // mapped any of them.
            //
            // The given bases and size are required to be multiples of `PAGE_SIZE`.
            fn map(
                    &self,
                    phys_base: usize,
                    virt_base: usize,
                    size: NonZeroUsize,
                    exception_level: ExceptionLevel,
                    asid: u16,
                    page_flags: PageEntry,
                    expected: PageEntry
            ) -> Result<(), Option<NonZeroUsize>> {
                let block_size = 1 << $bits_lo;
                let page_size = page_size();
                let bigger_block_size = 2 << $bits_hi;
                let size = size.get();
                let bigger_virt_base = virt_base & !(bigger_block_size - 1);
                let max_virt_bits = MAX_VIRT_BITS.load(Ordering::Acquire);
                let max_virt_bits_mask = (1 << (max_virt_bits as u64)) - 1;

                assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);
                assert_eq!(phys_base % page_size, 0, "{}", Text::PagesPhysBaseMisaligned(phys_base));
                assert_eq!(virt_base % page_size, 0, "{}", Text::PagesVirtBaseMisaligned(virt_base));
                assert_eq!(size % page_size, 0, "{}", Text::PagesSizeMisaligned(size));

                assert!(phys_base.checked_add(size).is_some(),
                    "physical address overflows address space (base: {:#018x}, size: {:#018x})", phys_base, size);
                let end_phys = phys_base + size;
                assert!(end_phys < 1 << MAX_PHYS_BITS.load(Ordering::Acquire),
                    "physical address {:#018x} overflows address space", end_phys);
                let mut addr_phys = phys_base;
                while addr_phys < end_phys {
                    let index = (virt_base + (addr_phys - phys_base) - bigger_virt_base) / block_size;
                    if $blocks_allowed && addr_phys % block_size == 0 && addr_phys + block_size <= end_phys {
                        // We can map a whole large page here.
                        let new_descriptor = Descriptor {
                            page: page_flags
                                | PageEntry::from_address(addr_phys as u64 & max_virt_bits_mask).unwrap()
                        };
                        match self.entries[index].compare_exchange(Descriptor { page: expected },
                                new_descriptor, Ordering::AcqRel, Ordering::Acquire) {
                            Ok(_) => {}, // Success!
                            Err(desc) => {
                                // At least one page has already been mapped in this block.
                                if let Some(mapped_size) = NonZeroUsize::new(addr_phys - phys_base) {
                                    self.unmap(virt_base, mapped_size, asid, expected);
                                }
                                if unsafe { desc.raw != new_descriptor.raw } {
                                    let next_addr = virt_base + (addr_phys - phys_base + block_size);
                                    if next_addr < 1 << max_virt_bits {
                                        return Err(NonZeroUsize::new(next_addr));
                                    } else {
                                        return Err(None);
                                    }
                                }
                            }
                        };
                    } else {
                        // This piece doesn't cover a whole block of size block_size, or block
                        // descriptors aren't allowed at this translation level. Delegate the task
                        // to a subtable.
                        // PERF: Keep a pool of allocated but unused subtables instead of
                        // constantly allocating new ones and freeing them when they turn out not
                        // to be needed. Or maybe a single thread-local cached table, since that's
                        // all any one thread will need.
                        let subtable_block = AllMemAlloc.malloc::<$next>(mem::size_of::<$next>(), NonZeroUsize::new(mem::align_of::<$next>()).unwrap())
                            .map_err(|AllocError| None)?;
                        unsafe {
                            <$next>::make_new(subtable_block.index(0));
                        }
                        // FIXME: We should cast through `PhysPtr` here, and throughout this file.
                        // We've assumed many times that the kernel's address space is
                        // identity-mapped.
                        let subtable_addr = subtable_block.index(0) as usize; 
                        let access_flags = match exception_level {
                            ExceptionLevel::El0 => PageTableEntry::PXN,
                            ExceptionLevel::El1 => PageTableEntry::UXN | PageTableEntry::EL1
                        };
                        let new_descriptor = Descriptor {
                            table: access_flags
                                | PageTableEntry::from_address(subtable_addr as u64).unwrap()
                                | PageTableEntry::ONE
                        };
                        let subsize = NonZeroUsize::new(
                            usize::min(end_phys - addr_phys, block_size - (addr_phys % block_size))
                        ).unwrap();
                        match self.entries[index].compare_exchange(Descriptor { table: PageTableEntry::UNMAPPED },
                                new_descriptor, Ordering::AcqRel, Ordering::Acquire) {
                            Ok(_) => {
                                let subtable = unsafe { &mut *subtable_block.index(0) };
                                if let Err(next_addr) = subtable.map(
                                        addr_phys,
                                        virt_base + (addr_phys - phys_base),
                                        subsize,
                                        exception_level,
                                        asid,
                                        page_flags,
                                        expected
                                ) {
                                    // Someone else mapped a page in this block before we could get to it.
                                    if let Some(mapped_size) = NonZeroUsize::new(addr_phys - phys_base) {
                                        self.unmap(virt_base, mapped_size, asid, expected);
                                    }
                                    return Err(next_addr);
                                }
                                mem::forget(subtable_block);
                            },
                            Err(desc) => {
                                unsafe {
                                    let subtable_addr = desc.table.address($granule_size) as usize;
                                    let subtable = &*(subtable_addr as *const $next);
                                    if let Err(next_addr) = subtable.map(
                                            addr_phys,
                                            virt_base + (addr_phys - phys_base),
                                            subsize,
                                            exception_level,
                                            asid,
                                            page_flags,
                                            expected
                                    ) {
                                        // At least one page has been mapped in this block.
                                        if let Some(mapped_size) = NonZeroUsize::new(addr_phys - phys_base) {
                                            self.unmap(virt_base, mapped_size, asid, expected);
                                        }
                                        return Err(next_addr);
                                    }
                                }
                            }
                        };
                    }
                    addr_phys += block_size;
                }
                Ok(())
            }

            // Unmaps all the pages in the given region.
            //
            // The given base and size are required to be multiples of `PAGE_SIZE`.
            fn unmap(&self, virt_base: usize, size: NonZeroUsize, asid: u16, new_entry: PageEntry) {
                let block_size = 1 << $bits_lo;
                let page_size = page_size();
                let bigger_block_size = 2 << $bits_hi;
                let size = size.get();
                let bigger_virt_base = virt_base & !(bigger_block_size - 1);

                assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);
                assert_eq!(virt_base % page_size, 0, "{}", Text::PagesVirtBaseMisaligned(virt_base));
                assert_eq!(size % page_size, 0, "{}", Text::PagesSizeMisaligned(size));

                assert!(virt_base.checked_add(size).is_some(),
                    "virtual address overflows address space (base: {:#018x}, size: {:#018x})", virt_base, size);
                let end_virt = virt_base + size;
                assert!(end_virt < 1 << MAX_PHYS_BITS.load(Ordering::Acquire),
                    "virtual address {:#018x} overflows address space", end_virt);
                let mut addr_virt = virt_base;
                while addr_virt < end_virt {
                    let index = (addr_virt - bigger_virt_base) / block_size;
                    let desc = self.entries[index].load(Ordering::AcqRel);
                    let subtable_entry = unsafe { desc.table };
                    if subtable_entry.contains(PageTableEntry::ONE) {
                        // Delegate the task to the subtable.
                        if let Some(size) = NonZeroUsize::new(usize::min(end_virt, addr_virt + block_size) - addr_virt) {
                            let subtable_ptr = subtable_entry.address($granule_size) as usize as *mut $next;
                            unsafe { (*subtable_ptr).unmap(addr_virt, size, asid, new_entry) };
                        }
                    } else if unsafe { desc.page.contains(PageEntry::ONE) } {
                        // Unmap the page.
                        match self.entries[index].compare_exchange(
                                desc,
                                Descriptor { page: new_entry },
                                Ordering::AcqRel,
                                Ordering::Acquire
                        ) {
                            Ok(_) => {
                                if new_entry == PageEntry::UNMAPPED {
                                    // FIXME: Decrement the parent's children count.
                                }
                                invalidate_tlb(addr_virt, asid);
                            },
                            Err(x) => {
                                panic!("tried to unmap a page that someone else was using: {:#x}", unsafe {
                                    x.page.bits()
                                });
                            }
                        };
                    } else if unsafe { desc.page.contains(PageEntry::IN_SWAPFILE) } {
                        // We somehow have a non-level-3 page in a swapfile? That's not currently supported.
                        panic!("non-level-3 page found in a swapfile");
                    } else if unsafe { desc.page.contains(PageEntry::IN_EXE_FILE) } {
                        // We somehow have a non-level-3 page in the executable file?
                        panic!("non-level-3 page found in an executable file");
                    }
                    addr_virt += block_size;
                }

                // TODO: Is it feasible to atomically unmap this table (and then drop it) if it's
                // now completely empty? Adding a counter to the struct would double its effective
                // size, so that's not worth the cost.
            }

            fn page_status(&self, virt_addr: usize) -> PageStatus {
                let block_size = 1 << $bits_lo;
                let bigger_block_size = 2 << $bits_hi;
                let bigger_virt_base = virt_addr & !(bigger_block_size - 1);

                let index = (virt_addr - bigger_virt_base) / block_size;
                let descriptor = self.entries[index].load(Ordering::Acquire);
                if unsafe { descriptor.table.contains(PageTableEntry::ONE) } {
                    unsafe {
                        let addr = descriptor.table.address($granule_size) as usize;
                        let phys_ptr = PhysPtr::<$next, *mut $next>::from_addr_phys(addr);
                        let subtable = phys_ptr.as_virt_unchecked();
                        (*subtable).page_status(virt_addr)
                    }
                } else if unsafe { descriptor.page.contains(PageEntry::ONE) } {
                    PageStatus::Mapped
                } else if unsafe { descriptor.page.contains(PageEntry::IN_SWAPFILE) } {
                    PageStatus::Mapped
                } else if unsafe { descriptor.page.contains(PageEntry::IN_EXE_FILE) } {
                    PageStatus::Mapped
                } else {
                    PageStatus::Unmapped
                }
            }

            fn location_in_swapfile(&self, virt_addr: usize) -> Option<u64> {
                let block_size = 1 << $bits_lo;
                let bigger_block_size = 2 << $bits_hi;
                let bigger_virt_base = virt_addr & !(bigger_block_size - 1);

                let index = (virt_addr - bigger_virt_base) / block_size;
                let descriptor = self.entries[index].load(Ordering::Acquire);
                if unsafe { descriptor.table.contains(PageTableEntry::ONE) } {
                    unsafe {
                        let addr = descriptor.table.address($granule_size) as usize;
                        let phys_ptr = PhysPtr::<$next, *mut $next>::from_addr_phys(addr);
                        let subtable = phys_ptr.as_virt_unchecked();
                        (*subtable).location_in_swapfile(virt_addr)
                    }
                } else if unsafe { descriptor.page.contains(PageEntry::ONE) } {
                    // This page is mapped in RAM, so it's not in the swapfile.
                    None
                } else if unsafe { descriptor.page.contains(PageEntry::IN_SWAPFILE) } {
                    // We somehow have a non-level-3 page in a swapfile? That's not currently supported.
                    panic!("non-level-3 page found in a swapfile");
                } else {
                    // This page might be in the executable file, but it's definitely not in the swapfile.
                    None
                }
            }

            fn max_entries() -> usize { 2 << ($bits_hi - $bits_lo) }
        }

        impl Drop for $table {
            fn drop(&mut self) {
                for entry in self.entries.iter() {
                    let descriptor = entry.load(Ordering::Acquire);

                    unsafe {
                        if descriptor.table.contains(PageTableEntry::ONE) {
                            // Drop and deallocate the subtable.
                            let addr = descriptor.table.address($granule_size) as usize;
                            let phys_ptr = PhysPtr::<$next, *mut $next>::from_addr_phys(addr);
                            AllMemAlloc.free(phys_ptr.as_virt_unchecked());
                        } else {
                            // TODO: If the block is in the swapfile, remove it from there.
                        }
                        // We don't have to drop mapped pages, since we only own the tables.
                    }
                }
            }
        }
    };
}

macro_rules! impl_leaf_table {
    ( $table:ty : bits $bits_lo:expr => $bits_hi:expr ) => {
        impl $table {
            unsafe fn make_new(table: *mut $table) {
                for entry in (*table).entries.iter() {
                    entry.store(PageEntry::UNMAPPED, Ordering::Release);
                }
            }

            // Calculates the amount of space that would be required for leaf translation tables in
            // order to identity-map the given regions.
            fn identity_map_size(regions: &[(usize, NonZeroUsize, RegionType, ShareabilityDomain)]) -> usize {
                // TODO: This function currently only works for one region at a time. But that's
                // fine for now because of how it's called.
                if regions.len() != 1 { unimplemented!(); }

                let base = regions[0].0;
                let size = regions[0].1;

                let bigger_block_size = 2 << $bits_hi;
                let base_index = base / bigger_block_size;
                let end_index = (base + size.get() + bigger_block_size - 1) / bigger_block_size;
                size_of::<$table>() * (end_index - base_index)
            }

            // Identity-maps the given range of addresses, except that it does not overwrite any pages that
            // have already been mapped. The given base and size are required to be multiples of `PAGE_SIZE`.
            fn identity_map_single_region(dest: &mut $table, base: usize, size: NonZeroUsize,
                    region_type: RegionType, shareability: ShareabilityDomain, subtable_addr: usize) -> usize {
                let block_size = 1 << $bits_lo;
                let page_size = page_size();
                let bigger_block_size = 2 << $bits_hi;
                let size = size.get();
                let bigger_base = base & !(bigger_block_size - 1);

                assert_eq!(page_size, block_size, "{}", Text::PageSizeDifferent(block_size, page_size));
                assert_eq!(base % page_size, 0, "{}", Text::PagesBaseMisaligned(base));
                assert_eq!(size % page_size, 0, "{}", Text::PagesSizeMisaligned(size));

                let mut addr = base & !(block_size - 1);
                let end = base + size;
                while addr < end {
                    let index = (addr - bigger_base) / block_size;

                    // Map the next page.
                    let type_flags = match region_type {
                        RegionType::Ram => PageEntry::normal_memory() | PageEntry::UXN | PageEntry::PXN,
                        RegionType::Rom => PageEntry::NOT_DIRTY | PageEntry::normal_memory(), // NOT_DIRTY = read-only in this context
                        RegionType::Mmio => PageEntry::device_memory()
                    };
                    let shareability_flags = match shareability {
                        ShareabilityDomain::NonShareable => PageEntry::empty(),
                        ShareabilityDomain::Inner => PageEntry::SHAREABLE | PageEntry::INNER,
                        ShareabilityDomain::Outer => PageEntry::SHAREABLE
                    };
                    let new_entry = PageEntry::UXN
                        | PageEntry::from_address(addr as u64).unwrap()
                        | PageEntry::ACCESSED
                        | shareability_flags
                        | type_flags
                        | PageEntry::LEVEL_3
                        | PageEntry::ONE;
                    match (*dest).entries[index].compare_exchange(PageEntry::UNMAPPED, new_entry, Ordering::AcqRel, Ordering::Acquire) {
                        Ok(_) => {}, // Success!
                        Err(_) => {} // Already mapped. Don't overwrite it.
                    };

                    addr += block_size;
                }

                subtable_addr
            }

            // Maps the given region to the given virtual base address.
            //
            // If any of these pages have already been mapped, the mapping fails without having
            // mapped any of them.
            //
            // The given bases and size are required to be multiples of `PAGE_SIZE`.
            fn map(
                    &self,
                    phys_base: usize,
                    virt_base: usize,
                    size: NonZeroUsize,
                    _exception_level: ExceptionLevel,
                    asid: u16,
                    mut page_flags: PageEntry,
                    expected: PageEntry
            ) -> Result<(), Option<NonZeroUsize>> {
                // FIXME: Use the ASID somehow.

                let block_size = 1 << $bits_lo;
                let page_size = page_size();
                let bigger_block_size = 2 << $bits_hi;
                let size = size.get();
                let bigger_virt_base = virt_base & !(bigger_block_size - 1);

                if page_flags.contains(PageEntry::ONE) {
                    page_flags |= PageEntry::LEVEL_3;
                }

                assert_eq!(page_size, block_size, "{}", Text::PageSizeDifferent(block_size, page_size));
                assert_eq!(phys_base % page_size, 0, "{}", Text::PagesPhysBaseMisaligned(phys_base));
                assert_eq!(virt_base % page_size, 0, "{}", Text::PagesVirtBaseMisaligned(virt_base));
                assert_eq!(size % page_size, 0, "{}", Text::PagesSizeMisaligned(size));

                assert!(phys_base.checked_add(size).is_some(),
                    "physical address overflows address space (base: {:#018x}, size: {:#018x})", phys_base, size);
                let end_phys = phys_base + size;
                assert!(end_phys < 1 << MAX_PHYS_BITS.load(Ordering::Acquire),
                    "physical address {:#018x} overflows address space", end_phys);
                let mut addr_phys = phys_base;
                while addr_phys < end_phys {
                    let index = (virt_base + (addr_phys - phys_base) - bigger_virt_base) / block_size;

                    // Map the next page.
                    let new_entry = page_flags
                        | PageEntry::from_address(addr_phys as u64).unwrap();
                    match self.entries[index].compare_exchange(expected, new_entry, Ordering::AcqRel, Ordering::Acquire) {
                        Ok(_) => {}, // Success!
                        Err(entry) => {
                            // This page is already mapped.
                            if let Some(mapped_size) = NonZeroUsize::new(addr_phys - phys_base) {
                                self.unmap(virt_base, mapped_size, asid, expected);
                            }
                            if entry != new_entry {
                                let next_addr = virt_base + (addr_phys - phys_base) + block_size;
                                if next_addr < 1 << MAX_VIRT_BITS.load(Ordering::Acquire) {
                                    return Err(NonZeroUsize::new(next_addr));
                                } else {
                                    return Err(None);
                                }
                            }
                        }
                    };

                    addr_phys += block_size;
                }
                Ok(())
            }

            // Unmaps all the pages in the given region.
            //
            // The given base and size are required to be multiples of `PAGE_SIZE`.
            fn unmap(&self, virt_base: usize, size: NonZeroUsize, asid: u16, new_entry: PageEntry) {
                let block_size = 1 << $bits_lo;
                let page_size = page_size();
                let bigger_block_size = 2 << $bits_hi;
                let size = size.get();
                let bigger_virt_base = virt_base & !(bigger_block_size - 1);

                assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);
                assert_eq!(virt_base % page_size, 0, "{}", Text::PagesVirtBaseMisaligned(virt_base));
                assert_eq!(size % page_size, 0, "{}", Text::PagesSizeMisaligned(size));

                let end_virt = virt_base + size;
                let mut addr_virt = virt_base;
                while addr_virt < end_virt {
                    let index = (addr_virt - bigger_virt_base) / block_size;
                    let entry = self.entries[index].swap(new_entry, Ordering::AcqRel);
                    if new_entry == PageEntry::UNMAPPED {
                        // FIXME: Decrement the parent's children count.
                    }
                    if entry.contains(PageEntry::ONE) {
                        invalidate_tlb(addr_virt, asid);
                    } else if entry.contains(PageEntry::IN_SWAPFILE) {
                        // TODO: Remove the page from the swapfile.
                    }
                    addr_virt += block_size;
                }

                // TODO: Is it feasible to atomically unmap this table (and then drop it) if it's
                // now completely empty? Adding a counter to the struct would double its effective
                // size, so that's not worth the cost.
            }

            fn page_status(&self, virt_addr: usize) -> PageStatus {
                let block_size = 1 << $bits_lo;
                let bigger_block_size = 2 << $bits_hi;
                let bigger_virt_base = virt_addr & !(bigger_block_size - 1);

                let index = (virt_addr - bigger_virt_base) / block_size;
                let entry = self.entries[index].load(Ordering::Acquire);
                if entry.contains(PageEntry::ONE | PageEntry::LEVEL_3) {
                    PageStatus::Mapped
                } else if entry.contains(PageEntry::IN_SWAPFILE) {
                    PageStatus::Mapped
                } else if entry.contains(PageEntry::IN_EXE_FILE) {
                    PageStatus::Mapped
                } else if entry == PageEntry::UNMAPPED {
                    PageStatus::Unmapped
                } else {
                    assert!(!entry.contains(PageEntry::ONE), "non-level-3 page entry found in a level-3 page table: {:#018x}", entry.bits());
                    PageStatus::TempUnmapped
                }
            }

            fn location_in_swapfile(&self, virt_addr: usize) -> Option<u64> {
                let block_size = 1 << $bits_lo;
                let bigger_block_size = 2 << $bits_hi;
                let bigger_virt_base = virt_addr & !(bigger_block_size - 1);

                let index = (virt_addr - bigger_virt_base) / block_size;
                let entry = self.entries[index].load(Ordering::Acquire);
                if entry.contains(PageEntry::ONE | PageEntry::LEVEL_3) {
                    // This page is mapped in RAM, so it's not in the swapfile.
                    None
                } else if entry.contains(PageEntry::IN_SWAPFILE) {
                    Some((entry & PageEntry::SWAPFILE_LOCATION).bits())
                } else {
                    // This page might be in the executable file, but it's definitely not in the swapfile.
                    None
                }
            }

            fn max_entries() -> usize { 2 << ($bits_hi - $bits_lo) }
        }
    };
}

fn invalidate_tlb(addr_virt: usize, asid: u16) {
    let arg = u64::try_from((addr_virt % (1 << MAX_VIRT_BITS.load(Ordering::Acquire))) >> 12).unwrap() | (u64::from(asid) << 48);
    unsafe {
        asm!(
            "tlbi vae1, {}",
            in(reg) arg,
            options(nostack, preserves_flags)
        );
    }
}

// Executes a Data Synchronization Barrier across the whole system.
fn dsb_sy() {
    unsafe { asm!("dsb sy", options(nostack, preserves_flags)) }
}

define_page_table!(Level0PageTable4k: Descriptor, 512; 0x1000);
define_page_table!(Level1PageTable4k: Descriptor, 512; 0x1000);
define_page_table!(Level2PageTable4k: Descriptor, 512; 0x1000);
define_page_table!(Level3PageTable4k: PageEntry, 512; 0x1000);

define_page_table!(Level0PageTable16k: Descriptor, 2; 0x40); // Alignment would be 0x10, but it must be at least 0x40 to support a 52-bit base address.
define_page_table!(Level1PageTable16k: Descriptor, 2048; 0x4000);
define_page_table!(Level2PageTable16k: Descriptor, 2048; 0x4000);
define_page_table!(Level3PageTable16k: PageEntry, 2048; 0x4000);

define_page_table!(Level1PageTable64k: Descriptor, 1024; 0x2000);
define_page_table!(Level2PageTable64k: Descriptor, 8192; 0x1_0000);
define_page_table!(Level3PageTable64k: PageEntry, 8192; 0x1_0000);

impl_branch_table!(Level0PageTable4k, Level1PageTable4k: bits 39 => 47; kiB 4; tables only);
impl_branch_table!(Level1PageTable4k, Level2PageTable4k: bits 30 => 38; kiB 4);
impl_branch_table!(Level2PageTable4k, Level3PageTable4k: bits 21 => 29; kiB 4);
impl_leaf_table!(Level3PageTable4k: bits 12 => 20);

impl_branch_table!(Level0PageTable16k, Level1PageTable16k: bits 47 => 47; kiB 16; tables only);
impl_branch_table!(Level1PageTable16k, Level2PageTable16k: bits 36 => 46; kiB 16; tables only);
impl_branch_table!(Level2PageTable16k, Level3PageTable16k: bits 25 => 35; kiB 16);
impl_leaf_table!(Level3PageTable16k: bits 14 => 24);

impl_branch_table!(Level1PageTable64k, Level2PageTable64k: bits 42 => 51; kiB 64; tables only); // Block descriptors allowed here iff ARMv8.2-LPA is implemented
impl_branch_table!(Level2PageTable64k, Level3PageTable64k: bits 29 => 41; kiB 64);
impl_leaf_table!(Level3PageTable64k: bits 16 => 28);

#[derive(Clone, Copy)]
#[repr(C)]
union Descriptor {
    table: PageTableEntry,
    page: PageEntry,
    raw: u64
}

impl Descriptor {
    fn raw(&self) -> u64 { unsafe { self.raw } }
}

impl From<u64> for Descriptor {
    fn from(v: u64) -> Descriptor {
        Descriptor { raw: v }
    }
}
impl From<Descriptor> for u64 {
    fn from(v: Descriptor) -> u64 {
        v.raw()
    }
}

impl fmt::Debug for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Descriptor {{raw: {:?}}}", self.raw())
    }
}

bitflags! {
    struct PageTableEntry: u64 {
        const UNMAPPED = 0x00000000_00000000;

        const NON_SECURE    = 0x80000000_00000000;
        const READ_ONLY     = 0x40000000_00000000;
        const EL1           = 0x20000000_00000000;
        const UXN           = 0x10000000_00000000; // Unprivileged Execute Never
        const PXN           = 0x08000000_00000000; // Privileged Execute Never
        const ADDRESS       = 0x0000ffff_fffff000; // Needs to be aligned to `PAGE_SIZE`
        const ADDRESS_48_51 = 0x00000000_0000f000; // 64-kiB granule only: bits 48-51 of the physical address
        const ONE           = 0x00000000_00000003; // Needs to be set for this to be an actual page table entry

        // Ignored bits we can use = 0x07f00000_00000ffc
        // The number of mapped children of this page table. It's split into two pieces because it needs 13 bits.
        const CHILDREN_COUNT_0_9   = 0x00000000_00000ffc;
        const CHILDREN_COUNT_10_12 = 0x00700000_00000000;
    }
}

bitflags! {
    struct PageEntry: u64 {
        const UNMAPPED = 0x00000000_00000000;

        const PBHA          = 0x78000000_00000000; // Used by the HW for implementation-defined purposes if we give permission.
        const UXN           = 0x00400000_00000000; // Unprivileged Execute Never
        const PXN           = 0x00200000_00000000; // Privileged Execute Never
        const CONTIGUOUS    = 0x00100000_00000000; // A caching hint: details on ARMv8-A Ref Manual p. 2122
        const DBM           = 0x00080000_00000000; // Dirty Bit Management (effectively means Writable and not CoW)
        const ADDRESS       = 0x0000ffff_fffff000; // Needs to be aligned to the size of the page
        const ADDRESS_48_51 = 0x00000000_0000f000; // 64-kiB granule only: bits 48-51 of the physical address
        // PERF: Use this bit (called "nG" in the ARM ARM) and ASIDs to avoid flushing the whole TLB with each context switch.
        const NOT_GLOBAL    = 0x00000000_00000800;
        const ACCESSED      = 0x00000000_00000400; // Records whether any read or write operation has been done on the page.
        const SHAREABLE     = 0x00000000_00000200;
        const INNER         = 0x00000000_00000100; // Combines with `SHAREABLE`. Should never be set if `SHAREABLE` isn't.
        const NOT_DIRTY     = 0x00000000_00000080; // The hardware sees this as a "read-only" flag, but we use it as an inverted dirty bit.
        const EL0           = 0x00000000_00000040;
        const NON_SECURE    = 0x00000000_00000020;
        const ATTR_INDEX    = 0x00000000_0000001c;
        const LEVEL_3       = 0x00000000_00000002; // Needs to be set in a level 3 table but not in any other table
        const ONE           = 0x00000000_00000001; // Needs to be set for this to be an actual page entry

        // Ignored bits we can use = 0x87800000_00000000
        const COW           = 0x00800000_00000000;

        const IN_SWAPFILE       = 0x00000000_00000002; // A page is in the swapfile if this is set and ONE is clear.
        const SWAPFILE_LOCATION = 0xffffffff_fffffffc;

        // A page is in the executable file if this is set and both ONE and IN_SWAPFILE are clear.
        const IN_EXE_FILE = 0x00000000_00000004;
    }
}

impl PageTableEntry {
    fn address(&self, granule_size: u8) -> u64 {
        let addr = (*self & Self::ADDRESS).bits();
        if granule_size == 64 {
            addr & !Self::ADDRESS_48_51.bits()
                | ((*self & Self::ADDRESS_48_51).bits() << 36)
        } else {
            addr
        }
    }

    fn from_address(addr: u64) -> Option<PageTableEntry> {
        assert_eq!(addr & !((1 << 52) - 1), 0, "{}", Text::AddrUsesTooManyBits(addr as usize, 52));
        let addr_0_47 = addr & ((1 << 48) - 1);
        let addr_48_51 = addr & !((1 << 48) - 1);
        PageTableEntry::from_bits(addr_0_47 | (addr_48_51 >> 36))
    }

    /*fn children_count(&self) -> u64 {
        let lo = (*self & Self::CHILDREN_COUNT_0_9).bits();
        let hi = (*self & Self::CHILDREN_COUNT_10_12).bits();
        (lo >> Self::CHILDREN_COUNT_0_9.bits().trailing_zeros())
            | (hi >> (Self::CHILDREN_COUNT_10_12.bits().trailing_zeros()
                      - Self::CHILDREN_COUNT_0_9.bits().count_ones()))
    }

    fn from_children_count(count: u64) -> Option<PageTableEntry> {
        let lo = (count << Self::CHILDREN_COUNT_0_9.bits().trailing_zeros()) & (Self::CHILDREN_COUNT_0_9).bits();
        let hi = (count << (Self::CHILDREN_COUNT_10_12.bits().trailing_zeros()
                            - Self::CHILDREN_COUNT_0_9.bits().count_ones()))
            & (Self::CHILDREN_COUNT_10_12).bits();
        let result = Self::from_bits(lo | hi)?;
        if (Self::CHILDREN_COUNT_0_9 | Self::CHILDREN_COUNT_10_12).contains(result) {
            Some(result)
        } else {
            None
        }
    }*/
}
impl PageEntry {
    fn normal_memory() -> PageEntry {
        PageEntry::ATTR_INDEX & PageEntry::from_bits((ATTR_NORMAL_MEMORY as u64) << 2).unwrap()
    }

    fn device_memory() -> PageEntry {
        PageEntry::ATTR_INDEX & PageEntry::from_bits((ATTR_DEVICE_MEMORY as u64) << 2).unwrap()
    }

    fn address(&self, granule_size: u8) -> u64 {
        let addr = (*self & Self::ADDRESS).bits();
        if granule_size == 64 {
            addr & !Self::ADDRESS_48_51.bits()
                | ((*self & Self::ADDRESS_48_51).bits() << 36)
        } else {
            addr
        }
    }

    fn from_address(addr: u64) -> Option<PageEntry> {
        assert_eq!(addr & !((1 << 52) - 1), 0, "{}", Text::AddrUsesTooManyBits(addr as usize, 52));
        let addr_0_47 = addr & ((1 << 48) - 1);
        let addr_48_51 = addr & !((1 << 48) - 1);
        PageEntry::from_bits(addr_0_47 | (addr_48_51 >> 36))
    }
}

impl From<u64> for PageTableEntry {
    fn from(v: u64) -> PageTableEntry {
        match PageTableEntry::from_bits(v) {
            Some(x) => x,
            None => panic!("{}", Text::PageTableEntryInvalid(v))
        }
    }
}
impl From<PageTableEntry> for u64 {
    fn from(v: PageTableEntry) -> u64 {
        v.bits()
    }
}
impl From<u64> for PageEntry {
    fn from(v: u64) -> PageEntry {
        match PageEntry::from_bits(v) {
            Some(x) => x,
            None => panic!("{}", Text::PageEntryInvalid(v))
        }
    }
}
impl From<PageEntry> for u64 {
    fn from(v: PageEntry) -> u64 {
        v.bits()
    }
}

/// Initializes the translation tables so that they identity-map the kernel with the .text and
/// .rodata segments set as read-only. Everything else is read-write, and all addresses that aren't
/// identified as RAM in the memory map are assumed to be for MMIO (and thus non-cacheable).
///
/// # Returns
/// A pointer to the root translation table.
#[no_mangle]
extern fn init_page_tables(page_size: usize, max_virt_bits: u8, max_phys_bits: u8) -> *const c_void {
    assert!(page_size.is_power_of_two(), "page size {:#x} is not a power of 2", page_size);
    assert!(max_virt_bits >= 48, "{}", Text::TooFewAddressableBits(48, max_virt_bits));
    assert!(max_virt_bits <= 52, "{}", Text::TooManyAddressableBits(52, max_virt_bits));
    assert!(max_phys_bits <= 52, "cannot support {}-bit physical addresses", max_phys_bits);

    // If the translation tables have already been made, return that pointer.
    match ROOT_PAGE_TABLE.load(Ordering::Acquire) {
        root_ptr if !root_ptr.is_null() => return root_ptr as *const c_void,
        _ => {}
    }

    // Save the page size and numbers of addressable bits for later.
    match PAGE_SIZE.compare_exchange(0, page_size, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {},
        Err(x) => assert_eq!(page_size, x)
    };
    match MAX_VIRT_BITS.compare_exchange(0, max_virt_bits, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {},
        Err(x) => assert_eq!(max_virt_bits, x)
    };
    match MAX_PHYS_BITS.compare_exchange(0, max_phys_bits, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {},
        Err(x) => assert_eq!(max_phys_bits, x)
    };

    // Allocate space for all the kernel's translation tables.
    let mut regions = Vec::new();

    // The system's ROM
    for region in MEMORY_MAP.present_regions().filter(|reg| reg.region_type == RegionType::Rom) {
        // If a region doesn't start or end at a page boundary, we'll treat it all as RAM. This
        // could lead to some bugs involving trying to write to ROM addresses, but memory safety
        // will still be intact. And other parts of the system, like the heap, might assume that
        // every byte of RAM in the memory map is writable.
        let padded_base = (region.base + (page_size - 1)) & !(page_size - 1);
        let padded_size = region.base.wrapping_add(region.size).wrapping_sub(padded_base) & !(page_size - 1);
        if let Some(padded_size) = NonZeroUsize::new(padded_size) {
            regions.push((padded_base, padded_size, RegionType::Rom, ShareabilityDomain::Outer));
        }
    }

    // The read-only part of the kernel will be mapped as if it were ROM and the read-write part as
    // RAM, regardless of what the memory map says. (At time of writing, the whole kernel is
    // excluded from the memory map.)
    let ro_base = readonly_start();
    assert_eq!(ro_base % page_size, 0, "{}", Text::KernelSymbolMisaligned("__readonly_start"));
    let ro_size = (readonly_end() - ro_base + (page_size - 1)) & !(page_size - 1);
    assert_eq!(ro_size % page_size, 0, "{}", Text::KernelSymbolMisaligned("__readonly_end"));
    if let Some(ro_size) = NonZeroUsize::new(ro_size) {
        regions.push((ro_base, ro_size, RegionType::Rom, ShareabilityDomain::Outer));
    }

    let rw_base = rw_shareable_start();
    assert!(ro_base + ro_size <= rw_base, "{}", Text::KernelRoOverlapsRw(rw_base - (ro_base + ro_size)));
    assert_eq!(rw_base % page_size, 0, "{}", Text::KernelSymbolMisaligned("__rw_shareable_start"));
    let rw_size = (rw_shareable_end() - rw_base + (page_size - 1)) & !(page_size - 1);
    assert_eq!(rw_size % page_size, 0, "{}", Text::KernelSymbolMisaligned("__rw_shareable_end"));
    if let Some(rw_size) = NonZeroUsize::new(rw_size) {
        regions.push((rw_base, rw_size, RegionType::Ram, ShareabilityDomain::Outer));
    }

    let nonshareable_base = rw_nonshareable_start();
    assert!(ro_base + ro_size <= nonshareable_base, "{}", Text::KernelRoOverlapsRw(nonshareable_base - (ro_base + ro_size)));
    assert_eq!(nonshareable_base % page_size, 0, "{}", Text::KernelSymbolMisaligned("__rw_nonshareable_start"));
    let nonshareable_size = (rw_nonshareable_end() - nonshareable_base + (page_size - 1)) & !(page_size - 1);
    assert_eq!(nonshareable_size % page_size, 0, "{}", Text::KernelSymbolMisaligned("__rw_nonshareable_end"));
    if let Some(nonshareable_size) = NonZeroUsize::new(nonshareable_size) {
        regions.push((nonshareable_base, nonshareable_size, RegionType::Ram, ShareabilityDomain::NonShareable));
    }

    // The rest of the system's RAM
    // PERF: Map the rest of the RAM lazily?
    for region in MEMORY_MAP.present_regions().filter(|reg| reg.region_type == RegionType::Ram) {
        // If a region doesn't start or end at a page boundary, we'll treat that part as uncacheable.
        // Everything else should be added as cacheable RAM.
        let padded_base = (region.base + (page_size - 1)) & !(page_size - 1);
        let padded_size = region.base.wrapping_add(region.size).wrapping_sub(padded_base) & !(page_size - 1);
        if let Some(padded_size) = NonZeroUsize::new(padded_size) {
            regions.push((padded_base, padded_size, RegionType::Ram, ShareabilityDomain::Outer));
        }
    }

    // Device memory (this also works for RAM but prevents caching).
    let addr_space_size = if page_size == 0x1_0000 {
        1usize << max_virt_bits
    } else {
        1usize << u8::min(max_virt_bits, 48)
    };
    if let Some(addr_space_size) = NonZeroUsize::new(addr_space_size) {
        regions.push((0, addr_space_size, RegionType::Mmio, ShareabilityDomain::Outer));
    }

    // Allocate and map all the necessary tables.
    let root_block = RootPageTable::identity_map(&regions[ .. ], KERNEL_ASID)
        .expect("failed to allocate the kernel's root page table");

    // Make sure everyone uses the same root table.
    let table_ptr;
    let root_ptr = root_block.index(0);
    match ROOT_PAGE_TABLE.compare_exchange(ptr::null_mut(), root_ptr, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            table_ptr = unsafe { &*root_ptr }.table_ptr();
            mem::forget(root_block);
        },
        Err(r) => {
            // Someone else made the translation tables first. Use those instead, and drop the ones
            // we made.
            table_ptr = unsafe { &*r }.table_ptr();
            drop(root_block);
        }
    };

    // PERF: Use PageEntry::CONTIGUOUS to optimize the TLB. ARMv8-A Reference Manual page 2122 has
    // the details on how to use it.

    table_ptr
}

/// Sets up a bare-bones set of translation tables that map the trampoline code to somewhere other
/// than its natural location, which is buried in the kernel's code.
///
/// # Returns
/// A pointer to the root translation table.
#[no_mangle]
extern fn init_trampoline_page_tables(max_virt_bits: u8) -> *const c_void {
    assert!(max_virt_bits >= 48, "{}", Text::TooFewAddressableBits(48, max_virt_bits));
    assert!(max_virt_bits <= 52, "{}", Text::TooManyAddressableBits(52, max_virt_bits));
    let max_virt_bits_mask = (1 << (max_virt_bits as u64)) - 1;

    // When mapping a page, this mask's clear bits must be clear in the address. When accessing the
    // page, they must be set in the address.
    let root_block = RootPageTable::new(ExceptionLevel::El1, KERNEL_ASID)
        .expect("failed to allocate the kernel trampoline's root page table");
    let root = unsafe { &mut *root_block.index(0) };
    let page_size = page_size();

    // Map the trampoline.
    let ro_size = (trampoline_ro_end() - trampoline_ro_start() + page_size - 1) / page_size * page_size;
    if let Some(ro_size) = NonZeroUsize::new(ro_size) {
        root.map(
            trampoline_ro_start(),
            Some((trampoline_virt() + (trampoline_ro_start() - trampoline_start())) & max_virt_bits_mask),
            ro_size,
            RegionType::Rom
        ).expect("failed to map the trampoline");
    }
    let rw_size = (trampoline_rw_end() - trampoline_rw_start() + page_size - 1) / page_size * page_size;
    if let Some(rw_size) = NonZeroUsize::new(rw_size) {
        root.map_dirty(
            trampoline_rw_start(),
            Some((trampoline_virt() + (trampoline_rw_start() - trampoline_start())) & max_virt_bits_mask),
            rw_size,
            RegionType::Ram
        ).expect("failed to map the trampoline");
    }

    // Allocate and map enough memory for each CPU to have its own little stack on the trampoline.
    let cpu_count = count_cpus();
    let size = (TRAMPOLINE_STACK_SIZE * cpu_count + page_size - 1) / page_size * page_size;
    let stacks_block = match AllMemAlloc.malloc::<[u8; TRAMPOLINE_STACK_SIZE]>(size, NonZeroUsize::new(page_size).unwrap()) {
        Ok(block) => {
            root.map_dirty(
                block.base().as_addr_phys(),
                Some(trampoline_stacks_virt() & max_virt_bits_mask),
                NonZeroUsize::new(size).unwrap(),
                RegionType::Ram
            ).expect("failed to map the trampoline");
            Some(block)
        },
        Err(AllocError) => {
            // Failing to allocate these stacks is catastrophic only if no one else has succeeded.
            if TRAMPOLINE_PAGE_TABLE.load(Ordering::SeqCst).is_null() {
                panic!("failed to allocate the trampoline stacks");
            }
            None
        }
    };

    // Make sure everyone uses the same root table.
    let table_ptr;
    drop(root); // We must avoid having two mutable references at once.
    let root_ptr = root_block.index(0);
    match TRAMPOLINE_PAGE_TABLE.compare_exchange(ptr::null_mut(), root_ptr, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            table_ptr = unsafe { &*root_ptr }.table_ptr();
            mem::forget(root_block);
            mem::forget(stacks_block);
        },
        Err(r) => {
            // Someone else made the translation tables first. Use those instead, and drop the ones
            // we made.
            table_ptr = unsafe { &*r }.table_ptr();
            drop(root_block);
            drop(stacks_block);
        }
    };

    table_ptr
}

/// A level of translation from physical to virtual addresses.
#[derive(Debug, Clone, Copy)]
pub enum TranslationLevel {
    /// Translation using a level-0 table (always a root if it exists).
    Level0,
    /// Translation using a level-1 table (may be a root, depending on translation granule size).
    Level1,
    /// Translation using a level-2 table.
    Level2,
    /// Translation using a level-3 table (which refers to individual pages).
    Level3
}

/// Attempts to resolve a Permission Fault caused by a write at the given virtual address. For
/// instance, if the address was marked as CoW, we'll copy it to another page and mark it as
/// dirty/writable.
pub fn resolve_write_fault(
        root: &RootPageTable,
        exc_level: ExceptionLevel,
        trans_level: TranslationLevel,
        addr: usize,
        access_size: NonZeroUsize
) -> Result<Option<BlockMut<u8>>, ()> {
    // An access spanning multiple pages is, as far as I can tell, impossible on ARM because of
    // alignment requirements. But since I'm not sure, we should check for that.
    let page_size = page_size();
    let last_addr = addr.wrapping_add(access_size.get()).wrapping_sub(1);
    assert_eq!(last_addr / page_size, addr / page_size,
        "access across page boundaries (address: {:#018x}, size: {:#018x})", addr, access_size.get());

    resolve_write_fault_byte(root, exc_level, trans_level, addr)
}

fn resolve_write_fault_byte(root: &RootPageTable, exc_level: ExceptionLevel, trans_level: TranslationLevel, addr: usize)
        -> Result<Option<BlockMut<u8>>, ()> {
    if let TranslationLevel::Level0 = trans_level {
        panic!("{}", Text::AddrTransLvlDoesntExist(0));
    }

    // This function determines whether the given page table entry forces the page that was
    // accessed to belong to the wrong exception level.
    let table_el_mismatch = |entry: PageTableEntry| {
        match exc_level {
            ExceptionLevel::El0 => entry.contains(PageTableEntry::EL1),
            ExceptionLevel::El1 => false // An EL0 page table can theoretically contain EL1 pages.
        }
    };

    // This function determines whether the given page entry puts the page that was accessed at the
    // wrong exception level, given that none of the page table entries forced it.
    let page_el_mismatch = |entry: PageEntry| {
        match exc_level {
            ExceptionLevel::El0 => !entry.contains(PageEntry::EL0),
            ExceptionLevel::El1 => entry.contains(PageEntry::EL0) // We don't allow the kernel to use EL0's virtual addresses.
        }
    };

    // TODO: Do this for each possible translation granule, using a macro.
    match root.internals {
        RootPageTableInternal::Table4k(ref _table1_block) => {
            // TODO
            Level0PageTable4k::max_entries();
            Level1PageTable4k::max_entries();
            Level2PageTable4k::max_entries();
            Level3PageTable4k::max_entries();
            unimplemented!()
        },
        RootPageTableInternal::Table16k(ref _table1_block) => {
            // TODO
            Level0PageTable16k::max_entries();
            Level1PageTable16k::max_entries();
            Level2PageTable16k::max_entries();
            Level3PageTable16k::max_entries();
            unimplemented!()
        },
        RootPageTableInternal::Table64k(ref table1_block) => {
            let index1 = (addr >> 42) % Level1PageTable64k::max_entries();
            let size1 = 1 << 42;
            let index2 = (addr >> 29) % Level2PageTable64k::max_entries();
            let size2 = 1 << 29;
            let index3 = (addr >> 16) % Level3PageTable64k::max_entries();
            let size3 = 1 << 16;

            // Level 1
            let table1 = unsafe { &*table1_block.index(0) };
            if let TranslationLevel::Level1 = trans_level {
                let mut descriptor = table1.entries[index1].load(Ordering::Acquire);
                loop {
                    // A Permission Fault should only ever be generated on a page (not a table),
                    // and only on one that is actually present. If that appears not to be the
                    // case, we have to assume another CPU resolved the fault first.
                    if unsafe { descriptor.table.contains(PageTableEntry::ONE) } {
                        return Ok(None);
                    }
                    let page = unsafe { descriptor.page };
                    if !page.contains(PageEntry::ONE) {
                        return Ok(None);
                    }

                    if page_el_mismatch(page) {
                        // The thread or kernel tried to access a page it doesn't own, or the
                        // page was unmapped and replaced with an unowned one. Either way, access
                        // is now denied.
                        return Err(());
                    }

                    if page.contains(PageEntry::DBM) {
                        // The page is writable and not CoW. Just clear the not-dirty bit.
                        match table1.entries[index1].compare_exchange_weak(
                            descriptor, 
                            Descriptor { page: page & !PageEntry::NOT_DIRTY },
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                invalidate_tlb(addr, root.asid);
                                return Ok(None);
                            },
                            Err(x) => descriptor = x
                        };
                    } else if page.contains(PageEntry::COW) {
                        // The page is CoW. Make a copy and remap the page to it.
                        // We have to unmap the page before remapping it to ensure that all CPUs'
                        // TLBs agree on its mapping.
                        let temp_unmapped = page & !PageEntry::ONE;
                        match table1.entries[index1].compare_exchange(
                            descriptor,
                            Descriptor { page: temp_unmapped },
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                // Ensure that the unmapped entry is flushed to memory.
                                dsb_sy();
                                // Ensure that all TLBs remove the unmapped entry.
                                invalidate_tlb(addr, root.asid);
                                dsb_sy();
                            },
                            Err(x) => {
                                descriptor = x;
                                continue;
                            }
                        };
                        // Now that we've temporarily unmapped the page, we should have exclusive
                        // access to its entry. Allocate, initialize, and map the new memory.
                        let block = AllMemAlloc.malloc::<u8>(size1, NonZeroUsize::new(size1).unwrap())
                            .map_err(|AllocError| ())?; // Out of memory!
                        let src = PhysPtr::<u8, *const u8>::from_addr_phys(page.address(64).try_into().unwrap()).as_virt_unchecked();
                        for i in 0 .. size1 {
                            unsafe {
                                *block.index(i) = *src.add(i);
                            }
                        }
                        let new_page = page & !PageEntry::COW & !PageEntry::NOT_DIRTY | PageEntry::DBM
                            & !PageEntry::ADDRESS | PageEntry::from_bits(block.get_ptr_phys(0).as_addr_phys().try_into().unwrap()).unwrap();
                        match table1.entries[index1].compare_exchange(
                            Descriptor { page: temp_unmapped },
                            Descriptor { page: new_page },
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                // Ensure that the mapped entry is flushed to memory before trying
                                // to use it.
                                dsb_sy();
                                return Ok(Some(block))
                            },
                            Err(_) => panic!("interference while working on temporarily unmapped page at {:#018x}", addr)
                        };
                    }
                }
            }

            // Level 2
            let entry = unsafe { table1.entries[index1].load(Ordering::Acquire).table };
            if !entry.contains(PageTableEntry::ONE) || table_el_mismatch(entry) {
                // The page we're looking for no longer exists, or it belongs to the wrong
                // exception level.
                return Err(());
            }
            let table2_addr = entry.address(64);
            let table2 = unsafe { &*(table2_addr as *const Level2PageTable64k) };
            if let TranslationLevel::Level2 = trans_level {
                let mut descriptor = table2.entries[index2].load(Ordering::Acquire);
                loop {
                    // A Permission Fault should only ever be generated on a page (not a table),
                    // and only on one that is actually present. If that appears not to be the
                    // case, we have to assume another CPU resolved the fault first.
                    if unsafe { descriptor.table.contains(PageTableEntry::ONE) } {
                        return Ok(None);
                    }
                    let page = unsafe { descriptor.page };
                    if !page.contains(PageEntry::ONE) {
                        return Ok(None);
                    }

                    if page_el_mismatch(page) {
                        // The thread or kernel tried to access a page it doesn't own, or the
                        // page was unmapped and replaced with an unowned one. Either way, access
                        // is now denied.
                        return Err(());
                    }

                    if page.contains(PageEntry::DBM) {
                        // The page is writable and not CoW. Just clear the not-dirty bit.
                        match table2.entries[index2].compare_exchange_weak(
                            descriptor, 
                            Descriptor { page: page & !PageEntry::NOT_DIRTY },
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                invalidate_tlb(addr, root.asid);
                                return Ok(None);
                            },
                            Err(x) => descriptor = x
                        };
                    } else if page.contains(PageEntry::COW) {
                        // The page is CoW. Make a copy and remap the page to it.
                        // We have to unmap the page before remapping it to ensure that all CPUs'
                        // TLBs agree on its mapping.
                        let temp_unmapped = page & !PageEntry::ONE;
                        match table2.entries[index2].compare_exchange(
                            descriptor,
                            Descriptor { page: temp_unmapped },
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                // Ensure that the unmapped entry is flushed to memory.
                                dsb_sy();
                                // Ensure that all TLBs remove the unmapped entry.
                                invalidate_tlb(addr, root.asid);
                                dsb_sy();
                            },
                            Err(x) => {
                                descriptor = x;
                                continue;
                            }
                        };
                        // Now that we've temporarily unmapped the page, we should have exclusive
                        // access to its entry. Allocate, initialize, and map the new memory.
                        let block = AllMemAlloc.malloc::<u8>(size2, NonZeroUsize::new(size2).unwrap())
                            .map_err(|AllocError| ())?; // Out of memory!
                        let src = PhysPtr::<u8, *const u8>::from_addr_phys(page.address(64).try_into().unwrap()).as_virt_unchecked();
                        for i in 0 .. size2 {
                            unsafe {
                                *block.index(i) = *src.add(i);
                            }
                        }
                        let new_page = page & !PageEntry::COW & !PageEntry::NOT_DIRTY | PageEntry::DBM
                            & !PageEntry::ADDRESS | PageEntry::from_bits(block.get_ptr_phys(0).as_addr_phys().try_into().unwrap()).unwrap();
                        match table2.entries[index2].compare_exchange(
                            Descriptor { page: temp_unmapped },
                            Descriptor { page: new_page },
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                // Ensure that the mapped entry is flushed to memory before trying
                                // to use it.
                                dsb_sy();
                                return Ok(Some(block))
                            },
                            Err(_) => panic!("interference while working on temporarily unmapped page at {:#018x}", addr)
                        };
                    }
                }
            }

            // Level 3
            let entry = unsafe { table2.entries[index2].load(Ordering::Acquire).table };
            if !entry.contains(PageTableEntry::ONE) || table_el_mismatch(entry) {
                // The page we're looking for no longer exists, or it belongs to the wrong
                // exception level.
                return Err(());
            }
            let table3_addr = entry.address(64);
            let table3 = unsafe { &*(table3_addr as *const Level3PageTable64k) };
            if let TranslationLevel::Level3 = trans_level {
                let mut entry = table3.entries[index3].load(Ordering::Acquire);
                loop {
                    // A Permission Fault should only ever be generated on a page that is actually
                    // present. If that appears not to be the case, we have to assume another CPU
                    // resolved the fault first.
                    if !entry.contains(PageEntry::ONE | PageEntry::LEVEL_3) {
                        return Ok(None);
                    }

                    if page_el_mismatch(entry) {
                        // The thread or kernel tried to access a page it doesn't own, or the
                        // page was unmapped and replaced with an unowned one. Either way, access
                        // is now denied.
                        return Err(());
                    }

                    if entry.contains(PageEntry::DBM) {
                        // The page is writable and not CoW. Just clear the not-dirty bit.
                        match table3.entries[index3].compare_exchange_weak(
                            entry, 
                            entry & !PageEntry::NOT_DIRTY,
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                invalidate_tlb(addr, root.asid);
                                return Ok(None);
                            },
                            Err(x) => entry = x
                        };
                    } else if entry.contains(PageEntry::COW) {
                        // The page is CoW. Make a copy and remap the page to it.
                        // We have to unmap the page before remapping it to ensure that all CPUs'
                        // TLBs agree on its mapping.
                        let temp_unmapped = entry & !PageEntry::ONE;
                        match table3.entries[index3].compare_exchange(
                            entry,
                            temp_unmapped,
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                // Ensure that the unmapped entry is flushed to memory.
                                dsb_sy();
                                // Ensure that all TLBs remove the unmapped entry.
                                invalidate_tlb(addr, root.asid);
                                dsb_sy();
                            },
                            Err(x) => entry = x
                        };
                        // Now that we've temporarily unmapped the page, we should have exclusive
                        // access to its entry. Allocate, initialize, and map the new memory.
                        let block = AllMemAlloc.malloc::<u8>(size3, NonZeroUsize::new(size3).unwrap())
                            .map_err(|AllocError| ())?; // Out of memory!
                        let src = PhysPtr::<u8, *const u8>::from_addr_phys(entry.address(64).try_into().unwrap()).as_virt_unchecked();
                        // PERF: As a special case, if `src == &*ZEROES_PAGE as *const u8`, we can
                        // fill the new block with zeroes rather than reading from the source page.
                        // That will avoid filling the cache with useless zeroes.
                        for i in 0 .. size3 {
                            unsafe {
                                *block.index(i) = *src.add(i);
                            }
                        }
                        let new_entry = entry & !PageEntry::COW & !PageEntry::NOT_DIRTY | PageEntry::DBM
                            & !PageEntry::ADDRESS | PageEntry::from_bits(block.get_ptr_phys(0).as_addr_phys().try_into().unwrap()).unwrap();
                        match table3.entries[index3].compare_exchange(
                            temp_unmapped,
                            new_entry,
                            Ordering::AcqRel,
                            Ordering::Acquire
                        ) {
                            Ok(_) => {
                                // Ensure that the mapped entry is flushed to memory before trying
                                // to use it.
                                dsb_sy();
                                return Ok(Some(block))
                            },
                            Err(_) => panic!("interference while working on temporarily unmapped page at {:#018x}", addr)
                        };
                    }
                }
            }

            unsafe {
                unreachable_debug!("Translation levels 0 through 3 have all been tested, and each branch returns or panics.")
            }
        }
    }
}

/// Sets the Accessed flag at the given translation level for the given virtual address. This
/// should only be called from the appropriate exception handler.
pub fn set_accessed_flag(root: Option<&RootPageTable>, level: TranslationLevel, addr: usize) {
    if let TranslationLevel::Level0 = level {
        panic!("{}", Text::AddrTransLvlDoesntExist(0));
    }

    // TODO: Do this for each possible translation granule, using a macro.
    let root = root.unwrap_or(unsafe { &*ROOT_PAGE_TABLE.load(Ordering::Acquire) });
    match root.internals {
        RootPageTableInternal::Table4k(ref _table1_block) => {
            // TODO
            unimplemented!()
        },
        RootPageTableInternal::Table16k(ref _table1_block) => {
            // TODO
            unimplemented!()
        },
        RootPageTableInternal::Table64k(ref table1_block) => {
            let index1 = (addr >> 42) % Level1PageTable64k::max_entries();
            let index2 = (addr >> 29) % Level2PageTable64k::max_entries();
            let index3 = (addr >> 16) % Level3PageTable64k::max_entries();

            // Level 1
            let table1 = unsafe { &*table1_block.index(0) };
            if let TranslationLevel::Level1 = level {
                table1.entries[index1].fetch_or(Descriptor { page: PageEntry::ACCESSED }, Ordering::AcqRel);
                invalidate_tlb(addr, root.asid);
                return;
            }

            // Level 2
            let entry = unsafe { table1.entries[index1].load(Ordering::Acquire).table };
            if !entry.contains(PageTableEntry::ONE) {
                // The page we're looking for no longer exists.
                return;
            }
            let table2_addr = entry.address(64);
            let table2 = unsafe { &*(table2_addr as *const Level2PageTable64k) };
            if let TranslationLevel::Level2 = level {
                table2.entries[index2].fetch_or(Descriptor { page: PageEntry::ACCESSED }, Ordering::AcqRel);
                invalidate_tlb(addr, root.asid);
                return;
            }

            // Level 3
            let entry = unsafe { table2.entries[index2].load(Ordering::Acquire).table };
            if !entry.contains(PageTableEntry::ONE) {
                // The page we're looking for no longer exists.
                return;
            }
            let table3_addr = entry.address(64);
            let table3 = unsafe { &*(table3_addr as *const Level3PageTable64k) };
            if let TranslationLevel::Level3 = level {
                table3.entries[index3].fetch_or(PageEntry::ACCESSED, Ordering::AcqRel);
                invalidate_tlb(addr, root.asid);
                return;
            }
        }
    }
}
