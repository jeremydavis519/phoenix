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

mod error;
mod headers;
mod segment;

use {
    core::{
        cmp::Ordering,
        mem::{self, MaybeUninit},
        num::NonZeroUsize,
        slice
    },
    alloc::{
        alloc::AllocError,
        sync::Arc,
        string::String,
        vec::Vec
    },

    locks::Mutex,

    i18n::Text,
    io::{Read, Seek, SeekFrom},
    memory::virt::paging::{self, RootPageTable},
    fs::File,

    super::{ExecImage, Segment, SegmentType},
    self::{
        error::ElfParseError,
        headers::*,
        segment::read_segment
    }
};

pub fn read_exe(file: File) -> io::Result<ExecImage<File>> {
    read_interpreter(file, None)
}

fn read_interpreter(mut file: File, interpreted: Option<File>) -> io::Result<ExecImage<File>> {
    file.seek(SeekFrom::Start(0))?;

    // Get the ELF header and confirm that we can use it.
    let elf_header: ElfHeader;
    unsafe {
        elf_header = read_struct(&mut file)?;
        elf_header.validate().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    }
    let elf_class = elf_header.class;
    let elf_header_ex = elf_header.ex_64();
    if elf_header_ex.ph_ent_size == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new(Text::ElfZeroSizedPH)));
    }

    // Make sure this is an executable file.
    if unsafe { elf_header_ex.file_type.common } != ElfTypeCommon::Exec {
        return Err(io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new(Text::ElfNotExecutable)));
    }

    let entry_point = elf_header_ex.entry_point as usize;
    let mut entry_point_in_segment = false;

    // A sorted array of segment descriptors
    let mut segments = Vec::with_capacity(elf_header_ex.ph_num as usize);

    // Handle the program header entries as they arise (rather than reading all of them at once).
    for i in 0 .. elf_header_ex.ph_num as u64 {
        file.seek(SeekFrom::Start(elf_header_ex.ph_off + i * elf_header_ex.ph_ent_size as u64))?;

        if let Some(segment) = read_segment(&mut file, elf_class)? {
            // Interpreter?
            if segment.seg_type == SegmentType::Interpreter {
                if interpreted.is_none() {
                    // Stop everything and load the interpreter instead.
                    // The path is a null-terminated string of maximum length `segment.file_sz`.
                    file.seek(SeekFrom::Start(segment.file_offset as u64))?;
                    let mut bytes = Vec::with_capacity(segment.file_sz);
                    for _ in 0 .. segment.file_sz {
                        let mut buffer = [0u8; 1];
                        file.read_exact(&mut buffer)?;
                        if buffer[0] == 0 {
                            break;
                        }
                        bytes.push(buffer[0]);
                    }
                    let path = String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    file.seek(SeekFrom::Start(0))?;
                    return read_interpreter(File::open(path.as_str())?, Some(file));
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new(Text::ElfInterpretedInterp)));
                }
            }

            // Dynamic section?
            if segment.seg_type == SegmentType::DLib {
                // TODO: Support dynamic linking.
                unimplemented!();
            }

            // Segment loaded into memory?
            if segment.mem_sz > 0 {
                let seg_cmp = |old: &Segment| {
                    // Considers two segments equal if they overlap in memory, which indicates an error.
                    if old.vaddr + old.mem_sz <= segment.vaddr {
                        Ordering::Less
                    } else if old.vaddr > segment.vaddr + segment.mem_sz {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                };
                if segment.vaddr <= entry_point && segment.vaddr + segment.mem_sz > entry_point {
                    entry_point_in_segment = true;
                }
                match segments.binary_search_by(seg_cmp) {
                    // Overlaps an existing segment
                    Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new(Text::ElfSegmentsOverlap))),
                    // No overlap so far
                    Err(index) => segments.insert(index, segment)
                };
            }
        }
    }

    // If the entry point isn't in any of the program's segments, the program can't be run.
    if !entry_point_in_segment {
        return Err(io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new(Text::ElfEntryPointNotInSegment)));
    }

    // TODO: Try to give each process its own ASID instead of using a constant one.
    const ASID: u16 = 0;

    let page_table = Arc::new(
        RootPageTable::new_userspace(ASID)
            .map_err(|AllocError| io::Error::new(io::ErrorKind::Other, AllocError))?
    );

    for segment in segments.iter() {
        let page_size = paging::page_size();
        let addr = segment.vaddr / page_size * page_size;
        let size = segment.vaddr.wrapping_add(segment.mem_sz).wrapping_sub(addr).wrapping_add(page_size - 1)
            / page_size * page_size;
        if let Some(size) = NonZeroUsize::new(size) {
            unsafe {
                (*page_table.index(0)).map_exe_file(Some(addr), size)
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, AllocError))?;
            }
        }
    }

    Ok(ExecImage {
        reader: Mutex::new(file),
        interpreted,
        entry_point,
        page_table,
        segments
    })
}

/*pub fn read_dlib(mut file: File) -> io::Result<DLibImage<File>> {
    // Get the ELF header and confirm that we can use it.
    let elf_header: ElfHeader;
    unsafe {
        elf_header = read_struct(&mut file)?;
        elf_header.validate().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    }
    let elf_header_ex = elf_header.ex_64();

    // Make sure this is a dynamic library.
    if unsafe { elf_header_ex.file_type.common } != ElfTypeCommon::Dyn {
        return Err(io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new(Text::ElfNotDlib)));
    }

    unimplemented!();
}*/

/// Reads any `Sized` structure from the reader's current location. This should only be used for
/// types with known memory layouts (i.e. those defined with `repr(C)` or `repr(transparent)`). On
/// failure, the structure is not dropped.
///
/// # Safety
/// This function is `unsafe` because it makes no guarantee that the returned structure is valid.
/// Using the structure without validating it first is undefined behavior.
unsafe fn read_struct<T: Read, U>(reader: &mut T) -> io::Result<U> {
    let mut result: MaybeUninit<U> = MaybeUninit::uninit();
    reader.read_exact(slice::from_raw_parts_mut(result.as_mut_ptr() as *mut u8, mem::size_of::<U>()))?;
    Ok(result.assume_init())
}

#[cfg(test)]
mod tests {
    // TODO: Add tests.
}
