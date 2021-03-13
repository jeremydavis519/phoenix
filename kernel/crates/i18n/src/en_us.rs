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

use core::fmt;
use core::panic::PanicInfo;
use alloc::string::String;

#[derive(Debug)]
pub enum Text<'a> {
    Aarch64UnrecognizedPhysAddrSize(&'a u64),
    AddrTransLvlDoesntExist(&'a usize),
    AddrUsesTooManyBits(&'a usize, &'a usize),
    CouldntReserveDeviceResource(&'a &'static str, &'a usize, &'a usize),
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
    KernelRoOverlapsRw(&'a usize),
    KernelSymbolMisaligned(&'a &'static str),
    LoadSegmentAllocErr(&'a usize, &'a usize),
    LoadSegmentOutOfBounds,
    MmioBusOutOfBounds(&'a usize, &'a usize, &'a usize, &'a usize),
    OutOfMemory(&'a usize, &'a usize),
    PageEntryInvalid(&'a u64),
    PageSizeDifferent(&'a usize, &'a usize),
    PageTableEntryInvalid(&'a u64),
    PagesBaseMisaligned(&'a usize),
    PagesPhysBaseMisaligned(&'a usize),
    PagesSizeMisaligned(&'a usize),
    PagesVirtBaseMisaligned(&'a usize),
    PhysBlockIndexOOB(&'a usize, &'a usize, &'a usize),
    ReadPastBuffer,
    PhoenixVersionHomepage(&'a Option<&'static str>, &'a Option<&'static str>),
    TooFewAddressableBits(&'a u8, &'a u8),
    TooManyAddressableBits(&'a u8, &'a u8),
    TriedToFreeNothing(&'a *const u8),
    TriedToShrinkNothing(&'a *const u8),
    Uart0CouldntReserveMmio,
    UnexpectedKernelError(&'a &'a PanicInfo<'a>)
}

impl<'a> Text<'a> {
    pub const fn couldnt_reserve_kernel()     -> &'static str { "failed to reserve the kernel's static memory" }
    pub const fn heap_block_node_not_followed_by_guard() -> &'static str {
        "expected a guard after the last block node in the heap but found none"
    }
    pub const fn heap_contains_no_guards()    -> &'static str { "heap contains no node guards" }
    pub const fn memory_map_not_retrieved()   -> &'static str { "failed to retrieve the memory map" }
    pub const fn too_little_memory_for_heap() -> &'static str { "ran out of memory while initializing the heap" }
    pub const fn unexpected_end_of_heap()     -> &'static str { "unexpected end of heap" }
    pub const fn unknown_version()            -> &'static str { "(unknown version)" }
}

impl<'a> fmt::Display for Text<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Text::Aarch64UnrecognizedPhysAddrSize(flags)
                                                  => write!(f, "unrecognized physical address size: ID_AA64MMFR0_EL1 = {:#x}", flags),
            Text::AddrTransLvlDoesntExist(level)  => write!(f, "address translation level {} does not exist", level),
            Text::AddrUsesTooManyBits(addr, expected_bits)
                                                  => write!(f, "address {:#x} uses more than {} bits", addr, expected_bits),
            Text::CouldntReserveDeviceResource(bus_type, base, size)
                                                  => write!(f, "failed to reserve a range of addresses starting at {:#x}, of size {:#x}, on bus type `{}`",
                                                        base, size, bus_type),
            Text::Elf32BitOn64Bit                 => write!(f, "32-bit binary on a 64-bit system"),
            Text::Elf64BitOn32Bit                 => write!(f, "64-bit binary on a 32-bit system"),
            Text::ElfArchExtFound                 => write!(f, "section of reserved type PT_AARCH64_ARCHEXT found"),
            Text::ElfBadSegAlign(align)           => write!(f, "segment alignment {:#x} is not a power of 2", align),
            Text::ElfBigOnLittle                  => write!(f, "big-endian binary on a little-endian system"),
            Text::ElfEntryPointNotInSegment       => write!(f, "entry point not in a segment"),
            Text::ElfHeaderTooSmall(expected, actual)
                                                  => write!(f, "ELF header too small (expected {} bytes; found {})", expected, actual),
            Text::ElfInterpretedInterp            => write!(f, "interpreter must not be interpreted"),
            Text::ElfInvalidFile(desc)            => write!(f, "invalid ELF file: {}", desc),
            Text::ElfInvalidSegmentFlags(val)     => write!(f, "invalid segment flags {:#x}", val),
            Text::ElfLittleOnBig                  => write!(f, "little-endian binary on a big-endian system"),
            Text::ElfNotDlib                      => write!(f, "not a dynamic library"),
            Text::ElfNotExecutable                => write!(f, "not an executable file"),
            Text::ElfPHEntriesTooSmall(expected, actual)
                                                  => write!(f, "program header entries too small (expected {} bytes; found {})", expected, actual),
            Text::ElfSegmentMisaligned(offset, vaddr)
                                                  => write!(f, "segment file offset {:#x} does not match image address {:#x}", offset, vaddr),
            Text::ElfSegmentsOverlap              => write!(f, "segments overlap in memory"),
            Text::ElfSHEntriesTooSmall(expected, actual)
                                                  => write!(f, "section header entries too small (expected {} bytes; found {})", expected, actual),
            Text::ElfShLibFound                   => write!(f, "section of reserved type PT_SHLIB found"),
            Text::ElfUnsupportedVersion(val)      => write!(f, "unsupported ELF version {:#x}", val),
            Text::ElfUnsupportedAbi(val)          => write!(f, "unsupported ABI {:#x}", val),
            Text::ElfUnsupportedArmAbi(val)       => write!(f, "unsupported ARM ABI {:#x}", val),
            Text::ElfUnsupportedArchitecture(val) => write!(f, "unsupported architecture {:#x}", val),
            Text::ElfUnsupportedEndianness(val)   => write!(f, "unsupported endianness {:#x}", val),
            Text::ElfUnsupportedFileType(val)     => write!(f, "unsupported ELF file type {:#x}", val),
            Text::ElfUnsupportedFlags(val)        => write!(f, "unsupported ELF flags {:#x}", val),
            Text::ElfUnsupportedPtrSize(val)      => write!(f, "unsupported pointer size (ELF class) {:#x}", val),
            Text::ElfUnsupportedSegmentType(val)  => write!(f, "unsupported segment type {:#x}", val),
            Text::ElfUnwindFound                  => write!(f, "section of reserved type PT_AARCH64_UNWIND found"),
            Text::ElfWrongMagicNumber(expected, actual)
                                                  => write!(f,
                                                        "wrong magic number (expected [{:#x}, {:#x}, {:#x}, {:#x}]; found [{:#x}, {:#x}, {:#x}, {:#x}])",
                                                        expected[0], expected[1], expected[2], expected[3],
                                                        actual[0], actual[1], actual[2], actual[3]
                                                    ),
            Text::ElfZeroSizedPH                  => write!(f, "program header has size 0"),
            Text::GicCouldntReserveCpuIntBlock    => write!(f, "failed to reserve the GIC CPU interface's memory-mapped I/O block"),
            Text::GicCouldntReserveDistBlock      => write!(f, "failed to reserve the GIC distributor's memory-mapped I/O block"),
            Text::GicIrqOutOfBounds(irq, max_irq) => write!(f, "attempted to register IRQ {}, but the GIC only supports IRQs up to {}", irq, max_irq),
            Text::GicReadUnreadableCpuIntReg(reg) => write!(f, "tried to read unreadable GIC CPU interface register {:#x}", reg),
            Text::GicReadUnreadableDistReg(reg)   => write!(f, "tried to read unreadable GIC distributor register {:#x}", reg),
            Text::GicWriteUnwritableCpuIntReg(reg)
                                                  => write!(f, "tried to write unwritable GIC CPU interface register {:#x}", reg),
            Text::GicWriteUnwritableDistReg(reg)  => write!(f, "tried to write unwritable GIC distributor register {:#x}", reg),
            Text::GpioCouldntReserveRegs          => write!(f, "failed to reserve the GPIO registers"),
            Text::HostedCouldntCloseFile(handle, errno)
                                                  => write!(f, "Phoenix kernel: could not close file {}: errno = {}", handle, errno),
            Text::IoErrAddrInUse                  => write!(f, "address in use"),
            Text::IoErrAddrNotAvailable           => write!(f, "address not available"),
            Text::IoErrAlreadyExists              => write!(f, "already exists"),
            Text::IoErrBrokenPipe                 => write!(f, "broken pipe"),
            Text::IoErrConnectionAborted          => write!(f, "connection aborted"),
            Text::IoErrConnectionRefused          => write!(f, "connection refused"),
            Text::IoErrConnectionReset            => write!(f, "connection reset"),
            Text::IoErrInterrupted                => write!(f, "interrupted"),
            Text::IoErrInvalidData                => write!(f, "invalid data"),
            Text::IoErrInvalidInput               => write!(f, "invalid input"),
            Text::IoErrNotConnected               => write!(f, "not connected"),
            Text::IoErrNotFound                   => write!(f, "file not found"),
            Text::IoErrOther                      => write!(f, "I/O error"),
            Text::IoErrPermissionDenied           => write!(f, "permission denied"),
            Text::IoErrTimedOut                   => write!(f, "timed out"),
            Text::IoErrUnexpectedEof              => write!(f, "unexpected end of file"),
            Text::IoErrWouldBlock                 => write!(f, "would block"),
            Text::IoErrWriteZero                  => write!(f, "write returned zero"),
            Text::KernelRoOverlapsRw(bytes)       => write!(f, "kernel's read-only segments overlap the read-write segments by {:#x} bytes", bytes),
            Text::KernelSymbolMisaligned(symbol)  => write!(f, "kernel symbol {} not aligned to a page boundary", symbol),
            Text::LoadSegmentAllocErr(base, size) => write!(f, "unable to allocate a new segment of size {1:#x} at address {0:#x}", base, size),
            Text::LoadSegmentOutOfBounds          => write!(f, "attempted to load a nonexistent part of a segment"),
            Text::MmioBusOutOfBounds(&base, &size, &parent_base, &parent_size) =>
                write!(f, "MMIO bus from {:#x} to {:#x} is out of bounds of parent memory region from {:#x} to {:#x}",
                    base, base.wrapping_add(size).wrapping_sub(1), parent_base, parent_base.wrapping_add(parent_size).wrapping_sub(1)),
            Text::OutOfMemory(size, align)        =>
                write!(f, "kernel ran out of memory trying to allocate memory with size = {:#x}, align = {:#x}", size, align),
            Text::PageEntryInvalid(entry)         => write!(f, "invalid page entry {:#x}", entry),
            Text::PageSizeDifferent(expected, actual)
                                                  => write!(f, "expected page size {:#x}; found {:#x}", expected, actual),
            Text::PageTableEntryInvalid(entry)    => write!(f, "invalid page table entry {:#x}", entry),
            Text::PagesBaseMisaligned(base)       => write!(f, "base address {:#x} is not page-aligned", base),
            Text::PagesPhysBaseMisaligned(base)   => write!(f, "physical base address {:#x} is not page-aligned", base),
            Text::PagesSizeMisaligned(size)       => write!(f, "size {:#x} is not page-aligned", size),
            Text::PagesVirtBaseMisaligned(base)   => write!(f, "virtual base address {:#x} is not page-aligned", base),
            Text::PhysBlockIndexOOB(base, size, index) =>
                write!(f, "physical memory block index out of bounds: block = {{ base: {:p}, size: {:#x} }}, index = {:#x}", base, size, index),
            Text::ReadPastBuffer                  => write!(f, "reader reported reading past the end of the buffer"),
            Text::PhoenixVersionHomepage(version, homepage) => {
                let result = write!(f, "Phoenix {}\n", version.unwrap_or(Text::unknown_version()))?;
                if let Some(homepage) = homepage {
                    write!(f, "{}\n", homepage)
                } else {
                    Ok(result)
                }
            },
            Text::TooFewAddressableBits(expected, actual)
                                                  => write!(f, "cannot support less than {}-bit addresses; found {}-bit", expected, actual),
            Text::TooManyAddressableBits(expected, actual)
                                                  => write!(f, "cannot support more than {}-bit addresses; found {}-bit", expected, actual),
            Text::TriedToFreeNothing(base)        => write!(f, "attempted to free a nonexistant block of memory at {:p}", base),
            Text::TriedToShrinkNothing(base)      => write!(f, "attempted to shrink a nonexistant block of memory at {:p}", base),
            Text::Uart0CouldntReserveMmio         => write!(f, "failed to reserve the UART0 MMIO block"),
            Text::UnexpectedKernelError(panic_info)
                                                  => write!(f, "Unexpected kernel error: {}", panic_info)
        }
    }
}
