/* Copyright (C) 2019-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate does nothing except store user-facing strings in all the languages the kernel
//! supports. Other aspects of internationalization, such as supporting different text directions,
//! are handled elsewhere.
//!
//! To print a string, simply use the `Display` property of the `Text` enum.

// TODO: This crate doesn't follow the Dependency Inversion principle. Everything depends on it, but
//       this crate is by far the most likely to change the most often. It would be best to have a
//       simple `i18n` crate that provided an interface to crates offering text in specific
//       languages, like `i18n-en-us` and `i18n-tok`. Each of these specific crates could
//       additionally depend on one more very small crate (`i18n-core`) that only defined the `Text`
//       enum as shown below. The dependency graph would look like this:
//
//                    +------------+
//             +------| i18n-en-us |----+           +------------+
//             v      +------------+    v           |            |
//       +-----------+              +------+        |   Other    |
//       | i18n-core |              | i18n |<-------|   Kernel   |
//       +-----------+              +------+        |   Crates   |
//             ^      +------------+    ^           |            |
//             +------|  i18n-tok  |----+           +------------+
//                    +------------+
//
//       This redesign would make it easy to add new languages and modify existing ones without
//       recompiling the whole kernel. Even adding a new user-facing string currently requires a
//       full recompilation, and that would no longer be the case.
//
//       The only difficulty is working out exactly how the interface will function.
//
//       New idea: Keep the `i18n` crate as it is, but don't list it in `Cargo.toml` as a
//       dependency in any other crate. Instead, compile this crate separately, statically link it
//       to the kernel, and use FFI to request a string. It won't be as fast as it currently is, but
//       since we're communicating with the user, we can afford to slow down. We'll also lose our
//       automatic type safety, but there may be a way to mostly restore it. I'm thinking of having
//       matching untagged `repr(u64)` `enum`s in both the kernel crates and the `i18n` crate,
//       except that each kernel crate only includes the variants that apply to it. (That way,
//       adding a string to one crate doesn't force them all to recompile.) This would offer FFI's
//       maximum possible type-safety. Then, `i18n` can return a `&'static dyn Display` that the
//       kernel can put on the screen.

// FIXME: Kernel panic messages don't need to be in here, since the user shouldn't see them anyway.
//        Instead, put a user-friendly generic panic message in here. In debug mode, the panic
//        handler should keep doing what it currently does. In release mode, it should print the
//        user-friendly message in the current language along with an automatically generated error
//        code based on kernel version, file, line, column, and panic message. The user can then
//        submit that error code in a bug report, and it will pinpoint the error for the developer.
//        For the average user, having the programmer-friendly panic message in their own language
//        doesn't help it be more understandable at all.

#![no_std]

#![deny(warnings, missing_docs)]

extern crate alloc;

#[macro_use] extern crate macros_unreachable;

use core::fmt;
use core::panic::PanicInfo;
use alloc::string::String;

#[macro_use] mod boilerplate;

boilerplate! {
    pub enum Language {
        // These languages should be ordered by decreasing priority. The first one in this list
        // that is compiled into the binary will be the one that the kernel uses from the start.
        EnUs: en_us: "en-us"; feature = "language_en_us"
        //Tok:  tok:   "tok";   feature = "language_tok"
    }

    pub enum Text<'a> {
        Aarch64UnrecognizedPhysAddrSize(flags: u64),
        AddrTransLvlDoesntExist(level: usize),
        AddrUsesTooManyBits(addr: usize, expected_bits: usize),
        CouldntReserveDeviceResource(bus_type: &'static str, base: usize, size: usize),
        Elf32BitOn64Bit,
        Elf64BitOn32Bit,
        ElfArchExtFound,
        ElfBadSegAlign(align: u64),
        ElfBigOnLittle,
        ElfEntryPointNotInSegment,
        ElfHeaderTooSmall(expected: usize, actual: u16),
        ElfInterpretedInterp,
        ElfInvalidFile(desc: String),
        ElfInvalidSegmentFlags(val: u32),
        ElfLittleOnBig,
        ElfNotDlib,
        ElfNotExecutable,
        ElfPHEntriesTooSmall(expected: usize, actual: u16),
        ElfSegmentMisaligned(offset: u64, vaddr: u64),
        ElfSegmentsOverlap,
        ElfSHEntriesTooSmall(expected: usize, actual: u16),
        ElfShLibFound,
        ElfUnsupportedVersion(val: u32),
        ElfUnsupportedAbi(val: u8),
        ElfUnsupportedArmAbi(val: u32),
        ElfUnsupportedArchitecture(val: u16),
        ElfUnsupportedEndianness(val: u8),
        ElfUnsupportedFileType(val: u16),
        ElfUnsupportedFlags(val: u32),
        ElfUnsupportedPtrSize(val: u8),
        ElfUnsupportedSegmentType(val: u32),
        ElfUnwindFound,
        ElfWrongMagicNumber(expected: [u8; 4], actual: [u8; 4]),
        ElfZeroSizedPH,
        GicCouldntReserveCpuIntBlock,
        GicCouldntReserveDistBlock,
        GicIrqOutOfBounds(irq: u64, max_irq: u64),
        GicReadUnreadableCpuIntReg(reg: usize),
        GicReadUnreadableDistReg(reg: usize),
        GicWriteUnwritableCpuIntReg(reg: usize),
        GicWriteUnwritableDistReg(reg: usize),
        GpioCouldntReserveRegs,
        HostedCouldntCloseFile(handle: String, errno: i64),
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
        KernelRoOverlapsRw(bytes: usize),
        KernelSymbolMisaligned(symbol: &'static str),
        LoadSegmentAllocErr(base: usize, size: usize),
        LoadSegmentOutOfBounds,
        MmioBusOutOfBounds(base: usize, size: usize, parent_base: usize, parent_size: usize),
        OutOfMemory(size: usize, align: usize),
        PageEntryInvalid(entry: u64),
        PageSizeDifferent(expected: usize, actual: usize),
        PageTableEntryInvalid(entry: u64),
        PagesBaseMisaligned(base: usize),
        PagesPhysBaseMisaligned(base: usize),
        PagesSizeMisaligned(size: usize),
        PagesVirtBaseMisaligned(base: usize),
        ReadPastBuffer,
        PhoenixVersionHomepage(version: Option<&'static str>, homepage: Option<&'static str>),
        TooFewAddressableBits(expected: u8, actual: u8),
        TooManyAddressableBits(expected: u8, actual: u8),
        TriedToFreeNothing(base: *const u8),
        TriedToShrinkNothing(base: *const u8),
        Uart0CouldntReserveMmio,
        UnexpectedKernelError(panic_info: &'a PanicInfo<'a>)
    }

    impl<'a> Text<'a> {
        pub fn couldnt_reserve_kernel() -> &'static str;
        pub fn heap_block_node_not_followed_by_guard() -> &'static str;
        pub fn heap_contains_no_guards() -> &'static str;
        pub fn memory_map_not_retrieved() -> &'static str;
        pub fn too_little_memory_for_heap() -> &'static str;
        pub fn unexpected_end_of_heap() -> &'static str;
        pub fn unknown_version() -> &'static str;
    }
}
