/* Copyright (c) 2018-2020 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the functions used to read and decode segments in an ELF file.

use {
    io::Read,
    super::{
        error::ElfParseError,
        headers::{
            ElfClass,
            ProgramHeaderEntry32,
            ProgramHeaderEntry64,
            SegmentFlags as ElfSegFlags
        },
        read_struct
    },
    crate::{Segment, SegmentType, SegmentFlags}
};
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
use super::headers::SegmentTypeArm as ElfSegTypeArm;
#[cfg(target_arch = "x86_64")]
use super::headers::SegmentTypeCommon as ElfSegTypeCommon;

pub(crate) fn read_segment<T: Read>(reader: &mut T, class: ElfClass) -> io::Result<Option<Segment>> {
    let ph_entry: ProgramHeaderEntry64;
    match class {
        ElfClass::Bits32 => {
            let temp_entry: ProgramHeaderEntry32;
            unsafe {
                temp_entry = read_struct(reader)?;
                temp_entry.validate().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            }
            ph_entry = temp_entry.into();
        },
        ElfClass::Bits64 => {
            unsafe {
                ph_entry = read_struct(reader)?;
                ph_entry.validate().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            }
        }
    };

    decode_segment(ph_entry)
}

#[cfg(target_arch = "aarch64")]
fn decode_segment(ph_entry: ProgramHeaderEntry64) -> io::Result<Option<Segment>> {
    // TODO: These flags might be segment-type-specific on some architectures.
    let flags = ph_entry.flags;
    let flags =
        if flags.contains(ElfSegFlags::EXECUTABLE) { SegmentFlags::EXECUTABLE } else { SegmentFlags::empty() } |
        if flags.contains(ElfSegFlags::WRITABLE) { SegmentFlags::WRITABLE } else { SegmentFlags::empty() } |
        if flags.contains(ElfSegFlags::READABLE) { SegmentFlags::READABLE } else { SegmentFlags::empty() };

    // TODO: Translate this into an architecture-independent, file-format-independent segment type.
    let seg_type = unsafe { ph_entry.seg_type.arm };
    match seg_type {
        ElfSegTypeArm::Null => Ok(None),
        
        ElfSegTypeArm::Load |
        ElfSegTypeArm::Interp |
        ElfSegTypeArm::Note |
        ElfSegTypeArm::PHdr => Ok(Some(Segment {
                seg_type:    SegmentType::Load,
                flags,
                file_offset: ph_entry.offset as usize,
                vaddr:       ph_entry.vaddr as usize,
                //paddr:       ph_entry.paddr as usize,
                file_sz:     ph_entry.file_sz as usize,
                mem_sz:      ph_entry.mem_sz as usize,
                //align:       ph_entry.align as usize
            })),

        ElfSegTypeArm::Dynamic => Ok(Some(Segment {
                seg_type:    SegmentType::DLib,
                flags,
                file_offset: ph_entry.offset as usize,
                vaddr:       ph_entry.vaddr as usize,
                //paddr:       ph_entry.paddr as usize,
                file_sz:     ph_entry.file_sz as usize,
                mem_sz:      ph_entry.mem_sz as usize,
                //align:       ph_entry.align as usize
            })),

        ElfSegTypeArm::ShLib => Err(
            io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new("section of reserved type PT_SHLIB found"))
        ),

        ElfSegTypeArm::ArchExt => Err(
            io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new("section of reserved type PT_AARCH64_ARCHEXT found"))
        ),

        ElfSegTypeArm::Unwind => Err(
            io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new("section of reserved type PT_AARCH64_UNWIND found"))
        ),

        // Segment types to ignore
        ElfSegTypeArm::GnuStack => { Ok(None) }
    }
}

#[cfg(target_arch = "x86_64")]
fn decode_segment(ph_entry: ProgramHeaderEntry64) -> io::Result<Option<Segment>> {
    // TODO: These flags might be segment-type-specific on some architectures.
    let flags = ph_entry.flags;
    let flags =
        if flags.contains(ElfSegFlags::EXECUTABLE) { SegmentFlags::EXECUTABLE } else { SegmentFlags::empty() } |
        if flags.contains(ElfSegFlags::WRITABLE) { SegmentFlags::WRITABLE } else { SegmentFlags::empty() } |
        if flags.contains(ElfSegFlags::READABLE) { SegmentFlags::READABLE } else { SegmentFlags::empty() };

    // TODO: Translate this into an architecture-independent, file-format-independent segment type.
    let seg_type = unsafe { ph_entry.seg_type.common };
    match seg_type {
        ElfSegTypeCommon::Null => Ok(None),
        
        ElfSegTypeCommon::Load |
        ElfSegTypeCommon::Interp |
        ElfSegTypeCommon::Note |
        ElfSegTypeCommon::PHdr => Ok(Some(Segment {
                seg_type:    SegmentType::Load,
                flags,
                file_offset: ph_entry.offset as usize,
                vaddr:       ph_entry.vaddr as usize,
                //paddr:       ph_entry.paddr as usize,
                file_sz:     ph_entry.file_sz as usize,
                mem_sz:      ph_entry.mem_sz as usize,
                //align:       ph_entry.align as usize
            })),

        ElfSegTypeCommon::Dynamic => Ok(Some(Segment {
                seg_type:    SegmentType::DLib,
                flags,
                file_offset: ph_entry.offset as usize,
                vaddr:       ph_entry.vaddr as usize,
                //paddr:       ph_entry.paddr as usize,
                file_sz:     ph_entry.file_sz as usize,
                mem_sz:      ph_entry.mem_sz as usize,
                //align:       ph_entry.align as usize
            })),

        ElfSegTypeCommon::ShLib => Err(
            io::Error::new(io::ErrorKind::InvalidData, ElfParseError::new("section of reserved type PT_SHLIB found"))
        ),

        // Segment types to ignore
        ElfSegTypeCommon::GnuStack => { Ok(None) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use super::super::headers::SegmentType as ElfSegType;
    use io::Read;

    macro_rules! assert_pat {
        ( $expr:expr, $($pat:tt)* ) => {
            match $expr {
                $($pat)* => {},
                x => panic!("found unexpected value {:?}", x)
            };
        };
    }

    struct SliceReader<'a>(&'a [u8]);
    impl Read for SliceReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let len = usize::min(self.0.len(), buf.len());
            for i in 0 .. usize::min(self.0.len(), buf.len()) {
                buf[i] = self.0[i];
            }
            self.0 = &self.0[len .. ];
            Ok(len)
        }
    }

    // TODO: Test some non-common segment types.
    #[test]
    fn read_null_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Null },
            offset: 0,
            vaddr: 0,
            paddr: 0,
            file_sz: 0,
            mem_sz: 0,
            flags: ElfSegFlags::empty(),
            align: 0
        }*/
        let mut reader = SliceReader(&[
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits32), Ok(None));
    }

    #[test]
    fn read_null_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Null },
            flags: ElfSegFlags::empty(),
            offset: 0,
            vaddr: 0,
            paddr: 0,
            file_sz: 0,
            mem_sz: 0,
            align: 0
        }*/
        let mut reader = SliceReader(&[
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits64), Ok(None));
    }

    #[test]
    fn read_runnable_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            offset:   2,
            vaddr:    3,
            paddr:    4,
            file_sz:  5,
            mem_sz:   6,
            flags:    ElfSegFlags::EXECUTABLE | ElfSegFlags::READABLE,
            align:    1
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00,
            0x06, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits32), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       x,
            file_offset: 2,
            vaddr:       3,
            file_sz:     5,
            mem_sz:      6
        })) if x == SegmentFlags::EXECUTABLE | SegmentFlags::READABLE);
    }

    #[test]
    fn read_runnable_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            flags:    ElfSegFlags::EXECUTABLE | ElfSegFlags::READABLE,
            offset:   2,
            vaddr:    3,
            paddr:    4,
            file_sz:  5,
            mem_sz:   6,
            align:    1
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits64), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       x,
            file_offset: 2,
            vaddr:       3,
            file_sz:     5,
            mem_sz:      6
        })) if x == SegmentFlags::EXECUTABLE | SegmentFlags::READABLE);
    }

    #[test]
    fn read_readonly_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            offset:   0x10000,
            vaddr:    0x20000,
            paddr:    0x30000,
            file_sz:  0x01567,
            mem_sz:   0x01567,
            flags:    ElfSegFlags::READABLE,
            align:    0x08000
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x03, 0x00,
            0x67, 0x15, 0x00, 0x00,
            0x67, 0x15, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,
            0x00, 0x80, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits32), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       SegmentFlags::READABLE,
            file_offset: 0x10000,
            vaddr:       0x20000,
            file_sz:     0x01567,
            mem_sz:      0x01567
        })));
    }

    #[test]
    fn read_readonly_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            flags:    ElfSegFlags::READABLE,
            offset:   0x10000,
            vaddr:    0x20000,
            paddr:    0x30000,
            file_sz:  0x01567,
            mem_sz:   0x01567,
            align:    0x08000
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x67, 0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x67, 0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits64), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       SegmentFlags::READABLE,
            file_offset: 0x10000,
            vaddr:       0x20000,
            file_sz:     0x01567,
            mem_sz:      0x01567
        })));
    }

    #[test]
    fn read_readwrite_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            offset:   0x047c00,
            vaddr:    0x007c00,
            paddr:    0xfedc00,
            file_sz:  0x000200,
            mem_sz:   0x001200,
            flags:    ElfSegFlags::READABLE | ElfSegFlags::WRITABLE,
            align:    0x010000
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x7c, 0x04, 0x00,
            0x00, 0x7c, 0x00, 0x00,
            0x00, 0xdc, 0xfe, 0x00,
            0x00, 0x02, 0x00, 0x00,
            0x00, 0x12, 0x00, 0x00,
            0x06, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits32), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       x,
            file_offset: 0x047c00,
            vaddr:       0x007c00,
            file_sz:     0x000200,
            mem_sz:      0x001200
        })) if x == SegmentFlags::READABLE | SegmentFlags::WRITABLE);
    }

    #[test]
    fn read_readwrite_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            flags:    ElfSegFlags::READABLE | ElfSegFlags::WRITABLE,
            offset:   0x047c00,
            vaddr:    0x007c00,
            paddr:    0xfedc00,
            file_sz:  0x000200,
            mem_sz:   0x001200,
            align:    0x010000
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x06, 0x00, 0x00, 0x00,
            0x00, 0x7c, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x7c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0xdc, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits64), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       x,
            file_offset: 0x047c00,
            vaddr:       0x007c00,
            file_sz:     0x000200,
            mem_sz:      0x001200
        })) if x == SegmentFlags::READABLE | SegmentFlags::WRITABLE);
    }

    #[test]
    fn read_writeonly_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            offset:   0x00000,
            vaddr:    0xa0000,
            paddr:    0xa0000,
            file_sz:  0x00000,
            mem_sz:   0x20000,
            flags:    ElfSegFlags::WRITABLE,
            align:    0x10000
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x0a, 0x00,
            0x00, 0x00, 0x0a, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x02, 0x00,
            0x02, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits32), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       SegmentFlags::WRITABLE,
            file_offset: 0x00000,
            vaddr:       0xa0000,
            file_sz:     0x00000,
            mem_sz:      0x20000
        })));
    }

    #[test]
    fn read_writeonly_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Load },
            flags:    ElfSegFlags::WRITABLE,
            offset:   0x00000,
            vaddr:    0xa0000,
            paddr:    0xa0000,
            file_sz:  0x00000,
            mem_sz:   0x20000,
            align:    0x10000
        }*/
        let mut reader = SliceReader(&[
            0x01, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits64), Ok(Some(Segment {
            seg_type:    SegmentType::Load,
            flags:       SegmentFlags::WRITABLE,
            file_offset: 0x00000,
            vaddr:       0xa0000,
            file_sz:     0x00000,
            mem_sz:      0x20000
        })));
    }

    #[test]
    fn read_dynamic_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Dynamic },
            offset:   0x00100000,
            vaddr:    0x48000000,
            paddr:    0x00000000,
            file_sz:  0x0000ffdc,
            mem_sz:   0x0000ffdc,
            flags:    ElfSegFlags::READABLE,
            align:    0x00010000
        }*/
        let mut reader = SliceReader(&[
            0x02, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0x00, 0x48,
            0x00, 0x00, 0x00, 0x00,
            0xdc, 0xff, 0x00, 0x00,
            0xdc, 0xff, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits32), Ok(Some(Segment {
            seg_type:    SegmentType::DLib,
            flags:       SegmentFlags::READABLE,
            file_offset: 0x00100000,
            vaddr:       0x48000000,
            file_sz:     0x0000ffdc,
            mem_sz:      0x0000ffdc
        })));
    }

    #[test]
    fn read_dynamic_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::Dynamic },
            flags:    ElfSegFlags::READABLE,
            offset:   0x00100000,
            vaddr:    0x48000000,
            paddr:    0x00000000,
            file_sz:  0x0000ffdc,
            mem_sz:   0x0000ffdc,
            align:    0x00010000
        }*/
        let mut reader = SliceReader(&[
            0x02, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xdc, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xdc, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert_pat!(read_segment(&mut reader, ElfClass::Bits64), Ok(Some(Segment {
            seg_type:    SegmentType::DLib,
            flags:       SegmentFlags::READABLE,
            file_offset: 0x00100000,
            vaddr:       0x48000000,
            file_sz:     0x0000ffdc,
            mem_sz:      0x0000ffdc
        })));
    }

    #[test]
    fn read_shlib_32() {
        /*ProgramHeaderEntry32 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::ShLib },
            offset:   0,
            vaddr:    0,
            paddr:    0,
            file_sz:  0,
            mem_sz:   0,
            flags:    ElfSegFlags::empty(),
            align:    0
        }*/
        let mut reader = SliceReader(&[
            0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ]);
        assert!(read_segment(&mut reader, ElfClass::Bits32).is_err());
    }

    #[test]
    fn read_shlib_64() {
        /*ProgramHeaderEntry64 {
            seg_type: ElfSegType { common: ElfSegTypeCommon::ShLib },
            flags:    ElfSegFlags::empty(),
            offset:   0,
            vaddr:    0,
            paddr:    0,
            file_sz:  0,
            mem_sz:   0,
            align:    0
        }*/
        let mut reader = SliceReader(&[
            0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ]);
        assert!(read_segment(&mut reader, ElfClass::Bits64).is_err());
    }
}
