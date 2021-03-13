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

//! This module defines the kernel's interface with the PL031, a real-time clock.

use {
    volatile::Volatile,

    memory::{
        allocator::AllMemAlloc,
        phys::block::Mmio
    },
    time::{Hertz, Femtosecs, hz_to_fs}
};

// The IRQ and MMIO range associated with the PL031.
// TODO: Instead of hard-coding these values, get them from something like ACPI.
attr! { #[cfg(target_machine = "qemu-virt")]
    // const IRQ: u64 = 18;
    const MMIO_BASE: usize = 0x0901_0000;
    const MMIO_SIZE: usize = 0x0000_1000;
}

lazy_static! {
    unsafe {
        /// The PL031's MMIO block.
        static ref MMIO: Mmio<Volatile<u32>> = {
            if let Ok(block) = AllMemAlloc.mmio_mut(MMIO_BASE, MMIO_SIZE) {
                block
            } else {
                panic!("failed to reserve the PL031's MMIO block");
            }
        };

        /// The underlying counter's frequency, measured in Hz.
        pub static ref COUNTER_FREQ: Hertz = Hertz(1);

        /// The counter's precision, measured in femtoseconds.
        pub static ref CLOCK_PRECISION: Femtosecs = hz_to_fs(*COUNTER_FREQ);
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum MmioRegs {
    RTCDR   = 0x000, // Data register
    RTCMR   = 0x004, // Match register
    RTCLR   = 0x008, // Load register
    RTCCR   = 0x00c, // Control register
    RTCIMSC = 0x010, // Interrupt Mask Set/Clear register
    RTCRIS  = 0x014, // Raw Interrupt Status register
    RTCMIS  = 0x018, // Masked Interrupt Status register
    RTCICR  = 0x01c, // Interrupt Clear register
    
    RTCPeriphID0 = 0xfe0, // Peripheral ID register bits [7:0]
    RTCPeriphID1 = 0xfe4, // Peripheral ID register bits [15:8]
    RTCPeriphID2 = 0xfe8, // Peripheral ID register bits [23:16]
    RTCPeriphID3 = 0xfec, // Peripheral ID register bits [31:24]
    
    RTCPCellID0 = 0xff0, // PrimeCell ID register bits [7:0]
    RTCPCellID1 = 0xff4, // PrimeCell ID register bits [15:8]
    RTCPCellID2 = 0xff8, // PrimeCell ID register bits [23:16]
    RTCPCellID3 = 0xffc  // PrimeCell ID register bits [31:24]
}

bitflags! {
    struct RtcCRFlags : u32 {
        const ENABLE = 0x1; // Setting to 1 enables the RTC. Setting to 0 is ignored.
    }
}

/// Initializes the PL031 (PrimeCell) RTC as a real-time clock.
pub fn init_clock_per_cpu() -> Result<(), ()> {
    // Verify that this is a PL031 designed by ARM Ltd (PeriphID[19:0] = 'A' 0x031).
    assert_eq!(get_periph_id() & 0xfffff, 0x41031);
    
    // Verify that the PrimeCell ID register agrees that this is a PL031.
    assert_eq!(get_primecell_id(), 0xb105f00d);
    
    unsafe { (*MMIO.index(MmioRegs::RTCCR as usize / 4)).write(RtcCRFlags::ENABLE.bits()); }

    Ok(())
}

/// Returns the number of clock ticks that have elapsed so far.
pub fn get_ticks_elapsed() -> u64 {
    unsafe { (*MMIO.index(MmioRegs::RTCDR as usize / 4)).read() as u64 }
}

fn get_periph_id() -> u32 {
    unsafe {
        ((*MMIO.index(MmioRegs::RTCPeriphID0 as usize / 4)).read() & 0xff) |
        (((*MMIO.index(MmioRegs::RTCPeriphID1 as usize / 4)).read() & 0xff) << 8) |
        (((*MMIO.index(MmioRegs::RTCPeriphID2 as usize / 4)).read() & 0xff) << 16) |
        (((*MMIO.index(MmioRegs::RTCPeriphID3 as usize / 4)).read() & 0xff) << 24)
    }
}

fn get_primecell_id() -> u32 {
    unsafe {
        ((*MMIO.index(MmioRegs::RTCPCellID0 as usize / 4)).read() & 0xff) |
        (((*MMIO.index(MmioRegs::RTCPCellID1 as usize / 4)).read() & 0xff) << 8) |
        (((*MMIO.index(MmioRegs::RTCPCellID2 as usize / 4)).read() & 0xff) << 16) |
        (((*MMIO.index(MmioRegs::RTCPCellID3 as usize / 4)).read() & 0xff) << 24)
    }
}

// TODO: Add an API to set a timer on the RTC, e.g. for when the user sets an alarm, or for thread-sleeping
//       on the order of seconds. At the lowest level, we can set the timer on the PL031 by programming RTCMR
//       (the RTC Match Register).
