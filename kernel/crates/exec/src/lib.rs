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

//! This module defines how the kernel reads and executes executable files.

#![no_std]

#![feature(allocator_api)]

#![deny(warnings, missing_docs)]

extern crate alloc;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate shared;

mod elf;
mod segment;

use {
    alloc::{
        alloc::AllocError,
        sync::Arc,
        vec::Vec
    },
    core::{
        convert::TryInto,
        fmt,
        num::NonZeroUsize,
        slice
    },

    locks::Mutex,

    error::Error,
    fs::File,
    io::{Read, Seek, SeekFrom},
    memory::{
        allocator::AllMemAlloc,
        phys::{
            RegionType,
            block::BlockMut
        },
        virt::paging::RootPageTable
    }
};
pub use segment::*;

/// Represents an executable file image in a file-format-independent form.
#[derive(Debug)]
pub struct ExecImage<T: Read+Seek> {
    reader: Mutex<T>,
    _interpreted: Option<T>, // If `Some`, this file is an interpreter and needs to be given this reader.
    /// The virtual address of the program's entry point.
    pub entry_point: usize,
    page_table: Arc<BlockMut<RootPageTable>>,
    segments: Vec<Segment>   // A sorted array of segment descriptors
}

/*/// Represents a dynamic library image in a file-format-independent form.
#[derive(Debug)]
pub struct DLibImage<T: Read+Seek> {
    reader: T,
    interpreted: Option<T>,
    segments: Vec<Segment>
}*/

/// Starts loading the executable from the given reader and returns an `ExecImage`. The segments
/// themselves aren't loaded yet: they're loaded lazily when the program is run by calling
/// `load_segment`.
pub fn read_exe(file: File) -> io::Result<ExecImage<File>> {
    // TODO: Cache the `ExecImage`. If another instance of the same program starts, it can skip
    // reading and validating all the file headers and, instead, clone the already-parsed image.
    // The only thing that can't be copied should be the root page table, although all the read-
    // only pages can be mapped to the same physical memory. (If we do that, we'll need to be
    // careful when swapping out pages.)
    elf::read_exe(file)
}

/*/// Starts loading the dynamic library from the given reader and returns a `DLibImage`. The
/// segments themselves aren't loaded yet: they're loaded lazily when the program is run by calling
/// `load_segment`.
pub fn read_dlib(file: File) -> io::Result<DLibImage<File>> {
    elf::read_dlib(file)
}*/

impl<T: Read+Seek> ExecImage<T> {
    /// Loads the given portion of the image into memory and maps it to virtual memory. More bytes
    /// may be loaded than requested. For instance, if the base and size aren't page-aligned, they
    /// will be modified to read entire pages.
    ///
    /// # Returns
    /// * `Ok(Some(block))` on a normal success
    /// * `Ok(None)` on success if the segment was mapped using a pre-existing block (e.g. for CoW)
    /// * `Err(Some(e))` on failure
    /// * `Err(None)` if the operation failed but should be retried later
    pub fn load_segment_piece(&self, base: usize, size: NonZeroUsize)
            -> Result<Option<BlockMut<u8>>, Option<LoadSegmentError>> {
        let segment = self.find_segment_containing(base, size.get())?;
        let page_size = memory::virt::paging::page_size();

        // Align the beginning and end of the requested piece.
        let end = base.wrapping_add(size.get()).wrapping_add(page_size - 1) / page_size * page_size;
        let base = base / page_size * page_size;
        let size = end.wrapping_sub(base);

        // If there's nothing to load from the file (e.g. this is a .bss section), just map a
        // pre-allocated CoW page filled with zeroes.
        let segment_overflows = segment.vaddr.checked_add(segment.file_sz).is_none();
        if !segment_overflows && base >= segment.vaddr + segment.file_sz {
            return self.page_table().map_zeroed_from_exe_file(base, NonZeroUsize::new(size).unwrap())
                .map(|()| None)
                .map_err(|()| Some(LoadSegmentError::MapError));
        }

        // PERF: If the pages are read-only and have already been loaded into another process with
        // the same `ExecImage`, share them instead of loading them from the reader again.

        // PERF: Load more than the bare minimum if more subsequent pages are likely to be needed.

        // Allocate enough space for the segment piece.
        let block = match AllMemAlloc.malloc::<u8>(size, NonZeroUsize::new(page_size).unwrap()) {
            Ok(block) => block,
            Err(AllocError) => return Err(Some(LoadSegmentError::AllocError(size)))
        };

        // Clear any bytes that won't come from the file.
        if base < segment.vaddr {
            let dest: &mut [u8] = unsafe { slice::from_raw_parts_mut(block.index(0), segment.vaddr - base) };
            dest.iter_mut().for_each(|x| *x = 0);
        }
        let block_overflows = base.checked_add(size).is_none();
        if !segment_overflows && (block_overflows || base + size > segment.vaddr + segment.file_sz) {
            let file_end = segment.vaddr + segment.file_sz;
            let dest: &mut [u8] = unsafe { slice::from_raw_parts_mut(block.index(file_end - base), end.wrapping_sub(file_end)) };
            dest.iter_mut().for_each(|x| *x = 0);
        }

        // Read any bytes in this segment piece that are contained in the file.
        if (segment_overflows || base < segment.vaddr + segment.file_sz)
                && (block_overflows || base + size > segment.vaddr) {
            let file_offset = segment.file_offset + base.saturating_sub(segment.vaddr);
            if let Ok(mut reader) = self.reader.try_lock() {
                reader.seek(SeekFrom::Start(file_offset.try_into().unwrap()))
                    .map_err(|e| Some(LoadSegmentError::IoError(e)))?;
                let buffer_base = usize::max(base, segment.vaddr);
                let buffer_size = usize::min(
                    base         .wrapping_add(size)           .wrapping_sub(buffer_base),
                    segment.vaddr.wrapping_add(segment.file_sz).wrapping_sub(buffer_base)
                );
                let buffer: &mut [u8] = unsafe { slice::from_raw_parts_mut(block.index(buffer_base - base), buffer_size) };
                reader.read_exact(buffer)
                    .map_err(|e| Some(LoadSegmentError::IoError(e)))?;
            } else {
                return Err(None);
            }
        }

        // Map the block into virtual memory.
        let region_type = if segment.flags.contains(SegmentFlags::WRITABLE) {
            RegionType::Ram
        } else {
            RegionType::Rom
        };
        self.page_table().map_from_exe_file(
                block.base().as_addr_phys(),
                base,
                NonZeroUsize::new(block.size()).unwrap(),
                region_type
        ).map_err(|()| LoadSegmentError::MapError)?;

        Ok(Some(block))
    }

    fn find_segment_containing(&self, base: usize, size: usize) -> Result<&Segment, LoadSegmentError> {
        self.segments.iter()
            .find(|seg| base >= seg.vaddr                      // Segment contains beginning
                && (seg.vaddr.checked_add(seg.mem_sz).is_none()
                    || base <= seg.vaddr + seg.mem_sz - size)) // Segment contains end (rearranged from b + s <= b + s to avoid overflow
            .ok_or(LoadSegmentError::OutOfBounds)
    }

    /// Borrows the root page table for the address space associated with this file's process.
    pub fn page_table(&self) -> &RootPageTable {
        unsafe { &*self.page_table.index(0) }
    }

    /// Makes a reader object that can seek to virtual addresses rather than to file offsets.
    pub fn virt_reader(&self) -> VirtReader<'_, T> {
        VirtReader { image: self, addr: 0 }
    }
}

/// An object that allows reading from an executable file by seeking to virtual memory addresses
/// within the file rather than to file offsets. This allows other crates to remain agnostic about
/// how executable images are mapped from files to memory.
#[derive(Debug, Copy)]
pub struct VirtReader<'a, T: Read+Seek> {
    image: &'a ExecImage<T>,
    addr: usize,
}

impl<'a, T: Read+Seek> Read for VirtReader<'a, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.image.reader.try_lock() {
            Ok(mut reader) => {
                reader.seek(SeekFrom::Start(
                    self.addr.try_into()
                        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?
                ))?;
                reader.read(buf)
            },
            Err(()) => Err(io::Error::from(io::ErrorKind::Interrupted)),
        }
    }
}

impl<'a, T: Read+Seek> Seek for VirtReader<'a, T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(pos) => self.addr = pos as usize,
            // Seeking from the end is interpreted as seeking from the top of the whole address space.
            // This is equivalent to seeking from the bottom of the address space.
            SeekFrom::End(offset) => self.addr = offset as usize,
            SeekFrom::Current(offset) => self.addr = self.addr.wrapping_add(offset as isize as usize),
        };
        Ok(self.addr as u64)
    }
}

impl<'a, T: Read+Seek> Clone for VirtReader<'a, T> {
    fn clone(&self) -> Self {
        Self { image: self.image, addr: self.addr }
    }
}

/// Represents an error that might occur when trying to load a segment of a program into memory.
#[derive(Debug)]
pub enum LoadSegmentError {
    /// No segment was found that contains every byte in the requested range.
    OutOfBounds,
    /// The heap failed to allocate space for the segment.
    AllocError(usize),
    /// The kernel failed to map the pages needed for the segment.
    MapError,
    /// An I/O error occurred.
    IoError(io::Error)
}

impl Error for LoadSegmentError {}

impl fmt::Display for LoadSegmentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadSegmentError::OutOfBounds => {
                write!(f, "attempted to load a nonexistent part of a segment")
            },
            LoadSegmentError::AllocError(size) => {
                write!(f, "unable to allocate a new segment piece of size {:#x}", size)
            },
            LoadSegmentError::MapError => {
                write!(f, "unable to map pages for the new segment piece")
            },
            LoadSegmentError::IoError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests.
}
