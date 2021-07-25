/* Copyright (c) 2018-2019 Jeremy Davis (jeremydavis519@gmail.com)
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

use {
    core::{
        convert::TryFrom,
        mem::size_of,
        ptr
    },
    i18n::Text,
    super::error::ElfParseError
};

//
// ---------- ELF Header ----------
//

#[repr(C, packed)]
pub(crate) struct ElfHeader {
    pub(crate) magic_number:  [u8; 4],
    pub(crate) class:         ElfClass,
    pub(crate) data:          ElfData,
    pub(crate) ident_version: u8,
    pub(crate) os_abi:        OsAbi,
    pub(crate) abi_version:   u8,
               padding:       [u8; 7],
    pub(crate) header_ex:     ElfHeaderEx
}

impl ElfHeader {
    /// Confirms that this ELF header could be correct (actual correctness depends on the contents
    /// of the rest of the file) and usable on this system.
    pub(crate) fn validate(&self) -> Result<(), ElfParseError> {
        if &self.magic_number != b"\x7fELF" {
            return Err(ElfParseError::new(Text::ElfWrongMagicNumber(*b"\x7fELF", self.magic_number.clone())));
        }

        ElfClass::validate(unsafe { *(&self.class as *const _ as *const u8) })?;
        ElfData::validate(unsafe { *(&self.data as *const _ as *const u8) })?;
        if self.ident_version != 1 {
            return Err(ElfParseError::new(Text::ElfUnsupportedVersion(self.ident_version as u32)));
        }
        OsAbi::validate(unsafe { *(&self.os_abi as *const _ as *const u8) }, self.abi_version)?;

        match self.class {
            ElfClass::Bits32 => unsafe { self.header_ex.header_32.validate()? },
            ElfClass::Bits64 => unsafe { self.header_ex.header_64.validate()? }
        };

        Ok(())
    }

    /// Returns the variable-sized portion of the ELF header as an `ElfHeaderEx64`, since the union
    /// type is more cumbersome.
    pub(crate) fn ex_64(self) -> ElfHeaderEx64 {
        match self.class {
            ElfClass::Bits32 => unsafe { self.header_ex.header_32 }.into(),
            ElfClass::Bits64 => unsafe { self.header_ex.header_64 }
        }
    }
}

#[repr(C)]
pub(crate) union ElfHeaderEx {
    header_32: ElfHeaderEx32,
    header_64: ElfHeaderEx64
}

#[repr(C, packed)]
pub(crate) struct ElfHeaderEx32 {
    file_type:     ElfType,
    target_arch:   Arch,
    elf_version:   u32,
    entry_point:   u32,
    ph_off:        u32,
    sh_off:        u32,
    flags:         ElfFlags,
    eh_size:       u16,
    ph_ent_size:   u16,
    ph_num:        u16,
    sh_ent_size:   u16,
    sh_num:        u16,
    sh_str_index:  u16
}

#[repr(C, packed)]
pub(crate) struct ElfHeaderEx64 {
    pub(crate) file_type:     ElfType,
    pub(crate) target_arch:   Arch,
    pub(crate) elf_version:   u32,
    pub(crate) entry_point:   u64,
    pub(crate) ph_off:        u64,
    pub(crate) sh_off:        u64,
    pub(crate) flags:         ElfFlags,
    pub(crate) eh_size:       u16,
    pub(crate) ph_ent_size:   u16,
    pub(crate) ph_num:        u16,
    pub(crate) sh_ent_size:   u16,
    pub(crate) sh_num:        u16,
    pub(crate) sh_str_index:  u16
}

impl From<ElfHeaderEx32> for ElfHeaderEx64 {
    fn from(header_32: ElfHeaderEx32) -> ElfHeaderEx64 {
        ElfHeaderEx64 {
            file_type:    header_32.file_type,
            target_arch:  header_32.target_arch,
            elf_version:  header_32.elf_version,
            entry_point:  header_32.entry_point as u64,
            ph_off:       header_32.ph_off as u64,
            sh_off:       header_32.sh_off as u64,
            flags:        header_32.flags,
            eh_size:      header_32.eh_size,
            ph_ent_size:  header_32.ph_ent_size,
            ph_num:       header_32.ph_num,
            sh_ent_size:  header_32.sh_ent_size,
            sh_num:       header_32.sh_num,
            sh_str_index: header_32.sh_str_index
        }
    }
}

impl ElfHeaderEx32 {
    pub(crate) fn validate(&self) -> Result<(), ElfParseError> {
        ElfType::validate(unsafe { *(ptr::addr_of!(self.file_type) as *const u16) })?;
        Arch::validate(unsafe { *(ptr::addr_of!(self.target_arch) as *const u16) })?;
        if self.elf_version != 1 {
            return Err(ElfParseError::new(Text::ElfUnsupportedVersion(self.elf_version)));
        }
        ElfFlags::validate(unsafe { *(ptr::addr_of!(self.flags) as *const u32) })?;
        if (self.eh_size as usize) < 16 + size_of::<ElfHeaderEx32>() {
            return Err(ElfParseError::new(Text::ElfHeaderTooSmall(16 + size_of::<ElfHeaderEx32>(), self.eh_size)));
        }
        if self.ph_ent_size != 0 && (self.ph_ent_size as usize) < size_of::<ProgramHeaderEntry32>() {
            return Err(ElfParseError::new(Text::ElfPHEntriesTooSmall(size_of::<ProgramHeaderEntry32>(), self.ph_ent_size)));
        }
        if self.sh_ent_size != 0 && (self.sh_ent_size as usize) < size_of::<SectionHeaderEntry32>() {
            return Err(ElfParseError::new(Text::ElfSHEntriesTooSmall(size_of::<SectionHeaderEntry32>(), self.sh_ent_size)));
        }

        Ok(())
    }
}

impl ElfHeaderEx64 {
    pub(crate) fn validate(&self) -> Result<(), ElfParseError> {
        ElfType::validate(unsafe { *(ptr::addr_of!(self.file_type) as *const u16) })?;
        Arch::validate(unsafe { *(ptr::addr_of!(self.target_arch) as *const u16) })?;
        if self.elf_version != 1 {
            return Err(ElfParseError::new(Text::ElfUnsupportedVersion(self.elf_version)));
        }
        ElfFlags::validate(unsafe { *(ptr::addr_of!(self.flags) as *const u32) })?;
        if (self.eh_size as usize) < 16 + size_of::<ElfHeaderEx64>() {
            return Err(ElfParseError::new(Text::ElfHeaderTooSmall(16 + size_of::<ElfHeaderEx64>(), self.eh_size)));
        }
        if self.ph_ent_size != 0 && (self.ph_ent_size as usize) < size_of::<ProgramHeaderEntry64>() {
            return Err(ElfParseError::new(Text::ElfPHEntriesTooSmall(size_of::<ProgramHeaderEntry64>(), self.ph_ent_size)));
        }
        if self.sh_ent_size != 0 && (self.sh_ent_size as usize) < size_of::<SectionHeaderEntry64>() {
            return Err(ElfParseError::new(Text::ElfSHEntriesTooSmall(size_of::<SectionHeaderEntry64>(), self.sh_ent_size)));
        }

        Ok(())
    }
}

#[repr(C)]
pub(crate) union ElfType {
    pub(crate) common: ElfTypeCommon
}

ffi_enum! {
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum ElfClass {
        Bits32 = 1,
        Bits64 = 2
    }

    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum ElfData {
        LittleEndian = 1,
        BigEndian = 2
    }

    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum OsAbi {
        SystemV       = 0x00,
        HpUx          = 0x01,
        NetBsd        = 0x02,
        Linux         = 0x03,
        GnuHurd       = 0x04,
        Solaris       = 0x06,
        Aix           = 0x07,
        Irix          = 0x08,
        FreeBsd       = 0x09,
        Tru64         = 0x0a,
        NovellModesto = 0x0b,
        OpenBsd       = 0x0c,
        OpenVms       = 0x0d,
        NonStopKernel = 0x0e,
        Aros          = 0x0f,
        FenixOs       = 0x10,
        CloudAbi      = 0x11
    }

    #[repr(u16)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum ElfTypeCommon {
        None   = 0x0000,
        Rel    = 0x0001,
        Exec   = 0x0002,
        Dyn    = 0x0003,
        Core   = 0x0004
        // LoOs .. HiOs = 0xfe00 .. 0xfeff
        // LoProc .. HiProc = 0xff00 .. 0xffff
    }

    #[repr(u16)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum Arch {
        None    = 0x0000,
        // Sparc   = 0x0002,
        X86     = 0x0003,
        // Mips    = 0x0008,
        // PowerPc = 0x0014,
        // S390    = 0x0016,
        Arm     = 0x0028,
        // SuperH  = 0x002a,
        // Ia64    = 0x0032,
        X86_64  = 0x003e,
        AArch64 = 0x00b7
        // RiscV   = 0x00f3
        // Note: There are more, but we don't need to list them unless we support them.
    }
}

#[cfg(target_arch = "arm")]
bitflags! {
    struct ElfFlags: u32 {
        const ABI_MASK = 0xff00_0000;
        const BE8      = 0x0080_0000;
        const GCC_MASK = 0x0040_0fff;
        const HW_FP    = 0x0000_0400;
        const SW_FP    = 0x0000_0200;
    }
}

#[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
bitflags! {
    pub(crate) struct ElfFlags: u32 {
        // AArch64 doesn't define any ELF header flags.
        const NO_FLAGS = 0;
    }
}

impl ElfClass {
    // For 64-bit targets that support 32-bit ELF files
    #[cfg(target_arch = "x86_64")]
    pub(crate) fn validate(val: u8) -> Result<(), ElfParseError> {
        match Self::try_from(val) {
            Ok(ElfClass::Bits32) => Ok(()),
            Ok(ElfClass::Bits64) => Ok(()),
            Err(_) => Err(ElfParseError::new(Text::ElfUnsupportedPtrSize(val)))
        }
    }

    // For 64-bit targets that only support 64-bit ELF files
    #[cfg(target_arch = "aarch64")]
    pub(crate) fn validate(val: u8) -> Result<(), ElfParseError> {
        // TODO: We should be able to use Aarch32 ELF files, right?
        match Self::try_from(val) {
            Ok(ElfClass::Bits32) => Err(ElfParseError::new(Text::Elf32BitOn64Bit)),
            Ok(ElfClass::Bits64) => Ok(()),
            Err(_) => Err(ElfParseError::new(Text::ElfUnsupportedPtrSize(val)))
        }
    }

    // For all 32-bit targets
    #[cfg(target_pointer_width = "32")]
    pub(crate) fn validate(val: u8) -> Result<(), ElfParseError> {
        match Self::try_from(val) {
            Ok(ElfClass::Bits32) => Ok(()),
            Ok(ElfClass::Bits64) => Err(ElfParseError::new(Text::Elf64BitOn32Bit)),
            Err(_) => Err(ElfParseError::new(Text::ElfUnsupportedPtrSize(val)))
        }
    }
}

impl ElfData {
    #[cfg(target_endian = "little")]
    pub(crate) fn validate(val: u8) -> Result<(), ElfParseError> {
        // If the file's endianness doesn't match the system's, we can't use it.
        // TODO: Some architectures, like AArch64, may allow software to change the system's
        // endianness. If the system supports that, we should support both endianness settings.
        // Also, ARMv6 supports BE-8 images, which seem to be a mixture of big- and
        // little-endian.
        match Self::try_from(val) {
            Ok(ElfData::LittleEndian) => Ok(()),
            Ok(ElfData::BigEndian)    => Err(ElfParseError::new(Text::ElfBigOnLittle)),
            Err(_) => Err(ElfParseError::new(Text::ElfUnsupportedEndianness(val)))
        }
    }

    #[cfg(target_endian = "big")]
    pub(crate) fn validate(val: u8) -> Result<(), ElfParseError> {
        // If the file's endianness doesn't match the system's, we can't use it.
        // TODO: Some architectures, like AArch64, may allow software to change the system's
        // endianness. If the system supports that, we should support both endianness settings.
        // Also, ARMv6 supports BE-8 images, which seem to be a mixture of big- and
        // little-endian.
        match Self::try_from(val) {
            Ok(ElfData::BigEndian)    => Ok(()),
            Ok(ElfData::LittleEndian) => Err(ElfParseError::new(Text::ElfLittleOnBig)),
            Err(_) => Err(ElfParseError::new(Text::ElfUnsupportedEndianness(val)))
        }
    }
}

impl OsAbi {
    pub(crate) fn validate(val: u8, _abi_version: u8) -> Result<(), ElfParseError> {
        if let Ok(_) = Self::try_from(val) {
            Ok(())
        } else {
            Err(ElfParseError::new(Text::ElfUnsupportedAbi(val)))
        }
    }
}

impl ElfType {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    pub(crate) fn validate(val: u16) -> Result<(), ElfParseError> {
        ElfTypeCommon::validate(val)
    }
}

impl ElfTypeCommon {
    pub(crate) fn validate(val: u16) -> Result<(), ElfParseError> {
        if let Ok(_) = Self::try_from(val) {
            Ok(())
        } else {
            Err(ElfParseError::new(Text::ElfUnsupportedFileType(val)))
        }
    }
}

impl Arch {
    pub(crate) fn validate(val: u16) -> Result<(), ElfParseError> {
        match Arch::try_from(val) {
            Ok(Arch::None) => Ok(()), // Made with no instruction set, so should be cross-platform
            Ok(Arch::X86)    if cfg!(target_arch = "x86")        => Ok(()),
            Ok(Arch::X86_64) if cfg!(target_arch = "x86_64")     => Ok(()),
            // Ok(Arch::Mips) if cfg!(target_arch = "mips") =>       Ok(()),
            // Ok(Arch::PowerPC) if cfg!(target_arch = "powerpc") => Ok(()),
            Ok(Arch::Arm) if cfg!(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))
                                                                 => Ok(()),
            Ok(Arch::AArch64) if cfg!(target_arch = "aarch64") => Ok(()),
            Ok(_)  => Err(ElfParseError::new(Text::ElfUnsupportedArchitecture(val))),
            Err(_) => Err(ElfParseError::new(Text::ElfUnsupportedArchitecture(val)))
        }
    }
}

impl ElfFlags {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
    pub(crate) fn validate(_flags: u32) -> Result<(), ElfParseError> {
        Ok(()) // No flags for AArch64
    }

    #[cfg(target_arch = "arm")]
    pub(crate) fn validate(flags: u32) -> Result<(), ElfParseError> {
        if let Some(flags) = ElfFlags::from_bits(flags) {
            if (flags & Self::ABI_MASK).bits() != 0x0500_0000 { // We only support ABI version 5 for now.
                Err(ElfParseError::new(Text::ElfUnsupportedArmAbi((flags & Self::ABI_MASK).bits())))
            } else {
                Ok(())
            }
        } else {
            Err(ElfParseError::new(Text::ElfUnsupportedFlags(flags)))
        }
    }
}

//
// ---------- Program Header ----------
//

/*#[repr(C)]
pub(crate) union ProgramHeaderEntry {
    ph_32: ProgramHeaderEntry32,
    ph_64: ProgramHeaderEntry64
}*/

#[repr(C, packed)]
pub(crate) struct ProgramHeaderEntry32 {
    seg_type: SegmentType,
    offset:   u32,
    vaddr:    u32,
    paddr:    u32,
    file_sz:  u32,
    mem_sz:   u32,
    flags:    SegmentFlags,
    align:    u32
}

#[repr(C, packed)]
pub(crate) struct ProgramHeaderEntry64 {
    pub(crate) seg_type: SegmentType,
    pub(crate) flags:    SegmentFlags,
    pub(crate) offset:   u64,
    pub(crate) vaddr:    u64,
    pub(crate) paddr:    u64,
    pub(crate) file_sz:  u64,
    pub(crate) mem_sz:   u64,
    pub(crate) align:    u64
}

impl From<ProgramHeaderEntry32> for ProgramHeaderEntry64 {
    fn from(old: ProgramHeaderEntry32) -> ProgramHeaderEntry64 {
        ProgramHeaderEntry64 {
            seg_type: old.seg_type,
            flags: old.flags,
            offset: old.offset as u64,
            vaddr: old.vaddr as u64,
            paddr: old.paddr as u64,
            file_sz: old.file_sz as u64,
            mem_sz: old.mem_sz as u64,
            align: old.align as u64
        }
    }
}

/*impl ProgramHeaderEntry {
    pub(crate) fn validate(&self, class: ElfClass) -> Result<(), ElfParseError> {
        match class {
            ElfClass::Bits32 => unsafe { self.ph_32.validate() },
            ElfClass::Bits64 => unsafe { self.ph_64.validate() }
        }
    }
}*/

impl ProgramHeaderEntry32 {
    pub(crate) fn validate(&self) -> Result<(), ElfParseError> {
        SegmentType::validate(unsafe { *(ptr::addr_of!(self.seg_type) as *const u32) })?;
        SegmentFlags::validate(unsafe { *(ptr::addr_of!(self.flags) as *const u32) })?;

        // The alignment should be 0 or a power of 2.
        if self.align.count_ones() > 1 {
            return Err(ElfParseError::new(Text::ElfBadSegAlign(self.align as u64)));
        }
        let align_mask = if self.align == 0 { 0 } else { self.align - 1 };

        if self.file_sz != 0 && self.mem_sz != 0 {
            // The addresses should be aligned correctly.
            if self.offset & align_mask != self.vaddr & align_mask {
                return Err(ElfParseError::new(Text::ElfSegmentMisaligned(self.offset as u64, self.vaddr as u64)));
            }
        }

        Ok(())
    }
}

impl ProgramHeaderEntry64 {
    pub(crate) fn validate(&self) -> Result<(), ElfParseError> {
        SegmentType::validate(unsafe { *(ptr::addr_of!(self.seg_type) as *const u32) })?;
        SegmentFlags::validate(unsafe { *(ptr::addr_of!(self.flags) as *const u32) })?;

        // The alignment should be 0 or a power of 2.
        if self.align.count_ones() > 1 {
            return Err(ElfParseError::new(Text::ElfBadSegAlign(self.align)));
        }
        let align_mask = if self.align == 0 { 0 } else { self.align - 1 };

        if self.file_sz != 0 && self.mem_sz != 0 {
            // The addresses should be aligned correctly.
            if self.offset & align_mask != self.vaddr & align_mask {
                return Err(ElfParseError::new(Text::ElfSegmentMisaligned(self.offset, self.vaddr)));
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
pub(crate) union SegmentType {
    pub(crate) common:  SegmentTypeCommon,
    pub(crate) arm:     SegmentTypeArm
}

impl SegmentType {
    #[cfg(target_arch = "x86_64")]
    pub(crate) fn validate(val: u32) -> Result<(), ElfParseError> {
        SegmentTypeCommon::validate(val)
    }

    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    pub(crate) fn validate(val: u32) -> Result<(), ElfParseError> {
        SegmentTypeArm::validate(val)
    }
}

// TODO: These flags might be segment-type-specific on some architectures.
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64",
          target_arch = "x86", target_arch = "x86_64"))]
bitflags! {
    pub(crate) struct SegmentFlags: u32 {
        const EXECUTABLE = 0x0000_0001;
        const WRITABLE  = 0x0000_0002;
        const READABLE   = 0x0000_0004;
        const MASK_OS    = 0x0ff0_0000;
        const MASK_PROC  = 0xf000_0000;
    }
}

impl SegmentFlags {
    fn validate(val: u32) -> Result<(), ElfParseError> {
        if SegmentFlags::from_bits(val).is_some() {
            Ok(())
        } else {
            Err(ElfParseError::new(Text::ElfInvalidSegmentFlags(val)))
        }
    }
}

ffi_enum! {
    #[repr(u32)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum SegmentTypeCommon {
        Null    = 0x0000_0000,
        Load    = 0x0000_0001,
        Dynamic = 0x0000_0002,
        Interp  = 0x0000_0003,
        Note    = 0x0000_0004,
        ShLib   = 0x0000_0005,
        PHdr    = 0x0000_0006,
        // LoOs .. HiOs = 0x6000_0000 .. 0x6fff_ffff
        // LoProc .. HiProc = 0x7000_0000 .. 0x7fff_ffff

        GnuEhFrame = 0x6474_e550,

        // Segment types to ignore
        GnuStack   = 0x6474_e551
    }

    #[repr(u32)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub(crate) enum SegmentTypeArm {
        Null    = SegmentTypeCommon::Null as u32,
        Load    = SegmentTypeCommon::Load as u32,
        Dynamic = SegmentTypeCommon::Dynamic as u32,
        Interp  = SegmentTypeCommon::Interp as u32,
        Note    = SegmentTypeCommon::Note as u32,
        ShLib   = SegmentTypeCommon::ShLib as u32,
        PHdr    = SegmentTypeCommon::PHdr as u32,
        ArchExt = 0x7000_0000,
        Unwind  = 0x7000_0001,

        GnuEhFrame = SegmentTypeCommon::GnuEhFrame as u32,

        // Segment types to ignore
        GnuStack   = SegmentTypeCommon::GnuStack as u32
    }
}

#[cfg(not(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64")))]
impl SegmentTypeCommon {
    pub(crate) fn validate(val: u32) -> Result<(), ElfParseError> {
        if Self::try_from(val).is_ok() {
            Ok(())
        } else {
            Err(ElfParseError::new(Text::ElfUnsupportedSegmentType(val)))
        }
    }
}

#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
impl SegmentTypeArm {
    pub(crate) fn validate(val: u32) -> Result<(), ElfParseError> {
        if Self::try_from(val).is_ok() {
            Ok(())
        } else {
            Err(ElfParseError::new(Text::ElfUnsupportedSegmentType(val)))
        }
    }
}

//
// ---------- Section Header ----------
//

/*#[repr(C)]
pub(crate) union SectionHeaderEntry {
    sh_32: SectionHeaderEntry32,
    sh_64: SectionHeaderEntry64
}*/

// TODO
#[repr(C, packed)]
pub(crate) struct SectionHeaderEntry32;

// TODO
#[repr(C, packed)]
pub(crate) struct SectionHeaderEntry64;

//
// ---------- Unit Tests ----------
//

#[cfg(test)]
mod tests {
    // TODO: Add tests.
}
