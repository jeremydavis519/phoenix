/* Copyright (c) 2019-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

use core::fmt;
use alloc::string::String;

#[derive(Debug)]
pub enum Text<'a> {
    Aarch64UnrecognizedPhysAddrSize(&'a u64),
    CouldntAllocateKernel,
    Elf32BitOn64Bit,
    Elf64BitOn32Bit,
    ElfArchExtFound,
    ElfBadSegAlign(&'a u64),
    ElfBigOnLittle,
    ElfEntryPointNotInSegment,
    ElfHeaderTooSmall(&'a usize, &'a u16),
    ElfInterpretedInterp,
    ElfInvalidFile(&'a String),
    ElfInvalidSegmentFlags(&'a u32),
    ElfLittleOnBig,
    ElfNotDlib,
    ElfNotExecutable,
    ElfPHEntriesTooSmall(&'a usize, &'a u16),
    ElfSegmentMisaligned(&'a u64, &'a u64),
    ElfSegmentsOverlap,
    ElfSHEntriesTooSmall(&'a usize, &'a u16),
    ElfShLibFound,
    ElfUnsupportedVersion(&'a u32),
    ElfUnsupportedAbi(&'a u8),
    ElfUnsupportedArmAbi(&'a u32),
    ElfUnsupportedArchitecture(&'a u16),
    ElfUnsupportedEndianness(&'a u8),
    ElfUnsupportedFileType(&'a u16),
    ElfUnsupportedFlags(&'a u32),
    ElfUnsupportedPtrSize(&'a u8),
    ElfUnsupportedSegmentType(&'a u32),
    ElfUnwindFound,
    ElfWrongMagicNumber(&'a [u8; 4], &'a [u8; 4]),
    ElfZeroSizedPH,
    ExcUnrecognizedInstSyndrome(&'a u32),
    ExcUnrecognizedSyndrome(&'a u32),
    FfiInvalidEnumVariant(&'a &'static str, &'a i128),
    GicCouldntReserveCpuIntBlock,
    GicCouldntReserveDistBlock,
    GicIrqOutOfBounds(&'a u64, &'a u64),
    GicReadUnreadableCpuIntReg(&'a usize),
    GicReadUnreadableDistReg(&'a usize),
    GicWriteUnwritableCpuIntReg(&'a usize),
    GicWriteUnwritableDistReg(&'a usize),
    GpioCouldntReserveRegs,
    HostedCouldntCloseFile(&'a String, &'a i64),
    IoErrAddrInUse,
    IoErrAddrNotAvailable,
    IoErrAlreadyExists,
    IoErrBrokenPipe,
    IoErrConnectionAborted,
    IoErrConnectionRefused,
    IoErrConnectionReset,
    IoErrInterrupted,
    IoErrInvalidData,
    IoErrInvalidInput,
    IoErrNotConnected,
    IoErrNotFound,
    IoErrOther,
    IoErrPermissionDenied,
    IoErrTimedOut,
    IoErrUnexpectedEof,
    IoErrWouldBlock,
    IoErrWriteZero,
    LoadSegmentAllocErr(&'a usize, &'a usize),
    LoadSegmentOutOfBounds,
    MemoryMapNotRetrieved,
    MmioBusOutOfBounds(&'a usize, &'a usize, &'a usize, &'a usize),
    OutOfMemory(&'a usize, &'a usize),
    ReadPastBuffer,
    PhoenixVersionHomepage(&'a Option<&'static str>, &'a Option<&'static str>),
    Uart0CouldntReserveMmio,
    VirtIoEnumOnNonMmioBus(&'a String)
}

// Translation notes. Make sure to follow and update these to keep the translations consistent:
//     English            => toki pona
//     -----------------------------------------
//     Abort              => pini
//     Address            => ma OR ma pi tomo sona
//     Alignment          => tomo ma pona
//     Architecture       => tomo
//     Big Endian         => open suli
//     Binary file        => lipu nanpa
//     Bit                => lili lili
//     Buffer             => ma sitelen
//     Bus                => nasin toki
//     Byte               => lili
//     Conflict           => utala
//     Distributor        => ilo pana
//     Dynamic library    => lipu nanpa kulupu
//     Entry (i.e. piece) => ijo lili
//     Entry point        => ma open
//     Enum               => nimi nanpa
//     Enumerate          => sitelen OR sitelen nanpa
//     Error              => pakala
//     Exception          => tenpo OR tenpo nasa (if unexpected) OR tenpo ike (if an error)
//     Executable file    => lipu pali
//     Expected           => mi wile X
//     File               => lipu
//     Flag               => palisa lawa
//     Found              => lukin
//     Header             => open
//     Index              => nanpa ma
//     Input              => pana insa
//     Interface          => ilo toki
//     Interpret          => pali
//     Interpretor        => ilo pi lipu pali
//     Little Endian      => open lili
//     Load               => lukin OR lukin sitelen
//     Magic number       => nanpa sewi
//     Memory             => tomo sona
//     Memory map         => lipu pi tomo sona
//     Multiple           => mute
//     Operating system   => poki lawa
//     Output             => pana pi insa ala
//     Physical           => kiwen
//     Pointer            => nasin palisa
//     Power (e.g. of 2)  => mute mute
//     Read               => lukin OR lukin sitelen
//     Refuse             => wile ala
//     Register           => lipu lili
//     Reserved           => mi li ken ala kepeken e X
//     Reset              => open sin
//     Section            => insa
//     Segment (runnable) => insa pali
//     Syndrome           => sitelen
//     System             => poki lawa
//     Type               => nasin tomo OR nasin
//     Unrecognized       => nasa
//     Version            => sijelo
//     Write              => sitelen

impl<'a> Text<'a> {
    pub fn unknown_version() -> &'static str { "(sijelo pi sona ala)" }
}

impl<'a> fmt::Display for Text<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Text::Aarch64UnrecognizedPhysAddrSize(flags)
                                                  => write!(f, "suli ni pi ma kiwen pi tomo sona li nasa: palisa lawa ID_AA64MMFR0_EL1 = {:#x}", flags),
            Text::CouldntAllocateKernel           => write!(f, "mi ken ala pali e tomo sona tawa poki lawa"),
            Text::Elf32BitOn64Bit                 => write!(f, "lipu nanpa pi lili lili 32 li lon poki lawa pi lili lili 64"),
            Text::Elf64BitOn32Bit                 => write!(f, "lipu nanpa pi lili lili 64 li lon poki lawa pi lili lili 32"),
            Text::ElfArchExtFound                 => write!(f, "mi lukin e insa PT_AARCH64_ARCHEXT li ken ala kepeken e ona"),
            Text::ElfBadSegAlign(align)           => write!(f, "tomo ma pona {:#x} pi insa pali li mute mute ala pi 2", align),
            Text::ElfBigOnLittle                  => write!(f, "lipu nanpa pi open suli li lon poki lawa pi open lili"),
            Text::ElfEntryPointNotInSegment       => write!(f, "ma open li lon ala insa pali"),
            Text::ElfHeaderTooSmall(expected, actual)
                                                  => write!(f, "open ELF li lili mute. (mi wile lili {} li lukin e lili {})", expected, actual),
            Text::ElfInterpretedInterp            => write!(f, "ijo li pali e ilo pi lipu pali"),
            Text::ElfInvalidFile(desc)            => write!(f, "lipu ELF li nasa: {}", desc),
            Text::ElfInvalidSegmentFlags(val)     => write!(f, "palisa lawa {:#x} pi insa pali li nasa", val),
            Text::ElfLittleOnBig                  => write!(f, "lipu nanpa pi open lili li lon poki lawa pi open suli"),
            Text::ElfNotDlib                      => write!(f, "ni li lipu pali kulupu ala"),
            Text::ElfNotExecutable                => write!(f, "ni li lipu pali ala"),
            Text::ElfPHEntriesTooSmall(expected, actual) => {
                write!(f, "ijo lili pi open pi insa pali li lili mute. (mi wile lili {} li lukin e lili {})", expected, actual)
            },
            Text::ElfSegmentMisaligned(offset, vaddr) => {
                write!(f, "ma pi insa pali lon lipu li {:#x}. ma pi ona lon tomo sona li {:#x}. tomo ma pona pi ona tu li ante", offset, vaddr)
            },
            Text::ElfSegmentsOverlap              => write!(f, "insa pali utala tawa ma sama"),
            Text::ElfSHEntriesTooSmall(expected, actual) => {
                write!(f, "ijo lili pi open insa li lili mute. (mi wile lili {} li lukin e lili {})", expected, actual)
            },
            Text::ElfShLibFound                   => write!(f, "mi lukin e insa PT_SHLIB li ken ala kepeken e ona"),
            Text::ElfUnsupportedVersion(val)      => write!(f, "sijelo ELF {:#x} li nasa", val),
            Text::ElfUnsupportedAbi(val)          => write!(f, "ijo ABI {:#x} li nasa", val),
            Text::ElfUnsupportedArmAbi(val)       => write!(f, "ijo ABI ARM {:#x} li nasa", val),
            Text::ElfUnsupportedArchitecture(val) => write!(f, "tomo {:#x} li nasa", val),
            Text::ElfUnsupportedEndianness(val)   => write!(f, "nasin open {:#x} li nasa", val),
            Text::ElfUnsupportedFileType(val)     => write!(f, "nasin tomo lipu ELF {:#x} li nasa", val),
            Text::ElfUnsupportedFlags(val)        => write!(f, "palisa lawa ELF {:#x} li nasa", val),
            Text::ElfUnsupportedPtrSize(val)      => write!(f, "suli pi nasin palisa (ijo ELF Class) {:#x} li nasa", val),
            Text::ElfUnsupportedSegmentType(val)  => write!(f, "nasin tomo pi insa pali {:#x} li nasa", val),
            Text::ElfUnwindFound                  => write!(f, "mi lukin e insa PT_AARCH64_UNWIND li ken ala kepeken e ona"),
            Text::ElfWrongMagicNumber(expected, actual)
                                                  => write!(f,
                                                        "nanpa sewi ike (mi wile [{:#x}, {:#x}, {:#x}, {:#x}] li lukin e [{:#x}, {:#x}, {:#x}, {:#x}])",
                                                        expected[0], expected[1], expected[2], expected[3],
                                                        actual[0], actual[1], actual[2], actual[3]
                                                    ),
            Text::ElfZeroSizedPH                  => write!(f, "suli pi open insa pali li 0"),
            Text::ExcUnrecognizedInstSyndrome(syndrome)
                                                  => write!(f, "sitelen tenpo pi toki pali (ESR_EL1.ISS) {:#x} li nasa", syndrome),
            Text::ExcUnrecognizedSyndrome(syndrome)
                                                  => write!(f, "sitelen tenpo (ESR_EL1) {:#x} li nasa", syndrome),
            Text::FfiInvalidEnumVariant(enum_type, value)
                                                  => write!(f, "mi ken ala pali e nimi nanpa {} kepeken nanpa {}", enum_type, value),
            Text::GicCouldntReserveCpuIntBlock    => write!(f, "mi ken ala pali e tomo sona pi ilo toki CPU GIC"),
            Text::GicCouldntReserveDistBlock      => write!(f, "mi ken ala pali e tomo sona pi ilo pana GIC"),
            Text::GicIrqOutOfBounds(irq, max_irq) => write!(f, "mi ken ala sitelen e tenpo pi nanpa {}. nanpa pi suli ali pi tenpo GIC li {}", irq, max_irq),
            Text::GicReadUnreadableCpuIntReg(reg) => write!(f, "mi ken ala pi lukin sitelen e lipu lili pi nanpa {:#x} pi ilo toki CPU GIC", reg),
            Text::GicReadUnreadableDistReg(reg)   => write!(f, "mi ken ala pi lukin sitelen e lipu lili pi nanpa {:#x} pi ilo pana GIC", reg),
            Text::GicWriteUnwritableCpuIntReg(reg)
                                                  => write!(f, "mi ken ala sitelen e lipu lili pi nanpa {:#x} pi ilo toki CPU GIC", reg),
            Text::GicWriteUnwritableDistReg(reg)  => write!(f, "mi ken ala sitelen e lipu lili pi nanpa {:#x} pi ilo pana GIC", reg),
            Text::GpioCouldntReserveRegs          => write!(f, "mi ken ala open jo e lipu lili pi nasin toki GPIO"),
            Text::HostedCouldntCloseFile(handle, errno)
                                                  => write!(f, "mi ken ala pini e lipu {}: nanpa Errno = {}", handle, errno),
            Text::IoErrAddrInUse                  => write!(f, "ijo li kepeken e ma"),
            Text::IoErrAddrNotAvailable           => write!(f, "ma li lon ala"),
            Text::IoErrAlreadyExists              => write!(f, "ni li lon"),
            Text::IoErrBrokenPipe                 => write!(f, "ijo li pakala e lupa"),
            Text::IoErrConnectionAborted          => write!(f, "ijo li pini e toki"),
            Text::IoErrConnectionRefused          => write!(f, "ijo li wile ala toki"),
            Text::IoErrConnectionReset            => write!(f, "ijo li open sin e toki"),
            Text::IoErrInterrupted                => write!(f, "mi ken ala pini"),
            Text::IoErrInvalidData                => write!(f, "nanpa li nasa"),
            Text::IoErrInvalidInput               => write!(f, "pana insa li nasa"),
            Text::IoErrNotConnected               => write!(f, "ni li toki ala"),
            Text::IoErrNotFound                   => write!(f, "mi lukin ala"),
            Text::IoErrOther                      => write!(f, "pakala I/O"),
            Text::IoErrPermissionDenied           => write!(f, "mi wile ala"),
            Text::IoErrTimedOut                   => write!(f, "tenpo suli"),
            Text::IoErrUnexpectedEof              => write!(f, "lipu li pini pi sona ala"),
            Text::IoErrWouldBlock                 => write!(f, "mi ni la mi awen"),
            Text::IoErrWriteZero                  => write!(f, "sitelen li pana e 0"),
            Text::LoadSegmentAllocErr(base, size) => write!(f, "ma {:#x} la mi ken ala pana e insa pali namako pi suli {:#x}", base, size),
            Text::LoadSegmentOutOfBounds          => write!(f, "ijo lili pi insa pali li lon ala. mi ken ala lukin e ona"),
            Text::MemoryMapNotRetrieved           => write!(f, "mi ken ala pali e lipu pi tomo sona"),
            Text::MmioBusOutOfBounds(&base, &size, &parent_base, &parent_size) =>
                write!(f, "nasin toki MMIO tan {:#x} tawa {:#x} li lon ala ma mama pi tomo sona tan {:#x} tawa {:#x}",
                    base, base.wrapping_add(size).wrapping_sub(1), parent_base, parent_base.wrapping_add(parent_size).wrapping_sub(1)),
            Text::OutOfMemory(size, align)        =>
                write!(f, "tan poki lawa li wile pali e tomo sona lon suli {:#x} lon tomo ma pona {:#x} la ona li jo ala e tomo sona lon tenpo ni",
                    size, align),
            Text::ReadPastBuffer                  => write!(f, "ilo pi lukin sitelen li pini a e ma sitelen"),
            Text::PhoenixVersionHomepage(version, homepage) => {
                let result = write!(f, "poki lawa Phoenix {}\n", version.unwrap_or(Text::unknown_version()))?;
                if let Some(homepage) = homepage {
                    write!(f, "{}\n", homepage)
                } else {
                    Ok(result)
                }
            },
            Text::Uart0CouldntReserveMmio         => write!(f, "mi ken ala open jo e nasin toki MMIO UART0"),
            Text::VirtIoEnumOnNonMmioBus(err)     => write!(f, "mi open lon nasin toki pi tomo sona ala la mi ken ala sitelen e ilo VirtIO: {}", err)
        }
    }
}

