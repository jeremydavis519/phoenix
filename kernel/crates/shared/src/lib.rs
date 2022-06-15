/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines helper functions, macros, and types for the rest of the kernel. It's
//! basically a stripped-down and specialized kind of standard library. As such, it cannot have
//! dependendies on any other crates in the kernel and is therefore pretty self-contained.
//!
//! TODO: It may be worth splitting this into many tiny crates. Most of the modules herein are
//! independent of each other, and a crate that depends on, for instance, the `ffi` module
//! shouldn't have to pull in something like the `rand` module as well unless that functionality is
//! also needed.

#![no_std]

#![feature(unsize)]
#![feature(coerce_unsized)]

#![deny(warnings, missing_docs)]

extern crate alloc;

#[cfg(target_arch = "aarch64")]
#[macro_use] extern crate bitflags;
#[macro_use] extern crate macros_unreachable;

use core::arch::asm;

pub mod ffi;
pub mod fs;
#[macro_use] pub mod once;
pub mod sync;

lazy_static! {
    unsafe {
        /// The current version of the kernel (defined in Cargo.toml)
        pub static ref KERNEL_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
        /// The homepage for the kernel's documentation (defined in Cargo.toml)
        pub static ref KERNEL_HOMEPAGE: Option<&'static str> = option_env!("CARGO_PKG_HOMEPAGE");
    }
}

/// Applies the same attributes to any number of items in order to reduce boilerplate code size.
#[macro_export]
macro_rules! attr {
    // Internal rules
    ( @attr_tuple $attrs:tt $($item:item)+ ) => { $(attr!(@expand $attrs $item);)+ };
    ( @expand ( $(#[$attr:meta]),+ ) $item:item ) => { $(#[$attr])+ $item };

    // The rule that should be used externally
    ( $(#[$attr:meta])+ $($item:item)+ ) => { attr!(@attr_tuple ($(#[$attr]),+) $($item)+); };
}

/// Puts the processor into a low-power state until an "event" happens (the definition depends on the architecture--see the
/// official description of the assembly instruction used for details). Or it might just return immediately. From the
/// software's perspective, this function does nothing, but it's sometimes needed for timing hardware accesses.
/// WARNING: No attempt is made to ensure that the event will ever happen at all. It should NEVER be called if events are
///          disabled (e.g. on x86 when IF is clear), unless the goal is to hang.
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
#[inline(always)]
pub fn wait_for_event() {
    unsafe {
        asm!(
            "dsb sy",
            "wfe",
            options(nomem, nostack, preserves_flags)
        );
    }
}
/// Puts the processor into a low-power state until an "event" happens (the definition depends on the architecture--see the
/// official description of the assembly instruction used for details). Or it might just return immediately. From the
/// software's perspective, this function does nothing, but it's sometimes needed for timing hardware accesses.
/// WARNING: No attempt is made to ensure that the event will ever happen at all. It should NEVER be called if events are
///          disabled (e.g. on x86 when IF is clear), unless the goal is to hang.
#[cfg(any(target_arch = "i386", target_arch = "i586", target_arch = "i686", target_arch = "x86_64"))]
#[inline(always)]
pub fn wait_for_event() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Puts the processor into a low-power state until an interrupt happens. Or it might just return immediately. From the
/// software's perspective, this function does nothing, but it's sometimes needed for timing hardware accesses.
/// WARNING: No attempt is made to ensure that the interrupt will ever happen at all. It should NEVER be called if interrupts are
///          disabled (e.g. on x86 when IF is clear), unless the goal is to hang.
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe {
        asm!(
            "dsb sy",
            "wfi",
            options(nomem, nostack, preserves_flags)
        );
    }
}
/// Puts the processor into a low-power state until an interrupt happens. Or it might just return immediately. From the
/// software's perspective, this function does nothing, but it's sometimes needed for timing hardware accesses.
/// WARNING: No attempt is made to ensure that the interrupt will ever happen at all. It should NEVER be called if interrupts are
///          disabled (e.g. on x86 when IF is clear), unless the goal is to hang.
#[cfg(any(target_arch = "i386", target_arch = "i586", target_arch = "i686", target_arch = "x86_64"))]
#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Disables all interrupts, or as many as can be disabled on the target architecture (e.g. software interrupts might still
/// be allowed).
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn disable_interrupts() {
    unsafe {
        asm!("msr DAIFSet, #0xf", options(nomem, nostack, preserves_flags));
    }
}
/// Disables all interrupts, or as many as can be disabled on the target architecture (e.g. software interrupts might still
/// be allowed).
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
#[inline(always)]
pub fn disable_interrupts() {
    unsafe {
        asm!("cpsid if", options(nomem, nostack, preserves_flags));
    }
}
/// Disables all interrupts, or as many as can be disabled on the target architecture (e.g. software interrupts might still
/// be allowed).
#[cfg(any(target_arch = "i386", target_arch = "i586", target_arch = "i686", target_arch = "x86_64"))]
#[inline(always)]
pub fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Enables all interrupts.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn enable_interrupts() {
    unsafe {
        asm!("msr DAIFClr, #0xf", options(nomem, nostack, preserves_flags));
    }
}
/// Enables all interrupts.
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
#[inline(always)]
pub fn enable_interrupts() {
    unsafe {
        asm!("cpsie if", options(nomem, nostack, preserves_flags));
    }
}
/// Enables all interrupts.
#[cfg(any(target_arch = "i386", target_arch = "i586", target_arch = "i686", target_arch = "x86_64"))]
#[inline(always)]
pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Returns the number of CPUs in the system.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn count_cpus() -> usize {
    // TODO
    1 // unimplemented!();
}
/// Returns the number of CPUs in the system.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn count_cpus() -> usize {
    // TODO
    unimplemented!();
}

/// Returns the index of this CPU, guaranteed to be in the range `0 .. count_cpus()`. This may be
/// different than the ID used by the hardware, but it is still guaranteed to be unique.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn cpu_index() -> usize {
    // TODO
    0 // unimplemented!();
}
/// Returns the index of this CPU, guaranteed to be in the range `0 .. count_cpus()`. This may be
/// different than the ID used by the hardware, but it is still guaranteed to be unique.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn cpu_index() -> usize {
    // TODO
    unimplemented!();
}

/// Returns the a number that represents this CPU for the purpose of affinity, such as making sure
/// a CPU prefers threads whose address spaces are already present in one or more of its caches.
/// This number can be attached to an object that prefers a particular CPU. Then, the numerical
/// difference between that number and the return value of this function indicates its compatibility
/// with this CPU. A difference closer to zero is a better fit.
#[cfg(target_arch = "aarch64")]
pub fn cpu_affinity() -> i64 {
    bitflags! {
        struct Mpidr: i64 {
            const AFFINITY     = 0x0000_00ff_00ff_ffff;
            const MULTITHREAD  = 0x0000_0000_0100_0000;
            const UNIPROCESSOR = 0x0000_0000_4000_0000;
            const RESERVED     = 0xffff_ff00_be00_0000_u64 as i64;

            const AFFINITY_0   = 0x0000_0000_0000_00ff;
            const AFFINITY_1   = 0x0000_0000_0000_ff00;
            const AFFINITY_2   = 0x0000_0000_00ff_0000;
            const AFFINITY_3   = 0x0000_00ff_0000_0000;
        }
    }

    let mpidr: i64;
    unsafe {
        asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nostack, nomem, preserves_flags));
    }
    mpidr & Mpidr::AFFINITY.bits()
}
/// Returns the a number that represents this CPU for the purpose of affinity, such as making sure
/// a CPU prefers threads whose address spaces are already present in one or more of its caches.
/// This number can be attached to an object that prefers a particular CPU. Then, the numerical
/// difference between that number and the return value of this function indicates its compatibility
/// with this CPU. A difference closer to zero is a better fit.
#[cfg(target_arch = "x86_64")]
pub fn cpu_affinity() -> i64 {
    // TODO
    unimplemented!();
}
