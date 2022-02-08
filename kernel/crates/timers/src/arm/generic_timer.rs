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

//! This module defines the kernel's interface with the ARM generic timer.

#![cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]

use {
    core::{
        arch::asm,
        convert::TryInto,
        mem
    },
    irqs::{self, IsrResult},
    time::{Duration, Hertz, Femtosecs, hz_to_fs},
    crate::Timer
};

// TODO: Instead of hard-coding these values, get them from something like ACPI.
/// The IRQ associated with the Generic Timer (specifically, the virtual timer).
#[cfg(target_machine = "qemu-virt")]
const IRQ: u64 = 27;

bitflags! {
    /// Bit flags for the Counter-timer Kernel Control Register
    struct CntkctlEl1: u32 {
        const EL0PCTEN = 0x001; // Trap accesses to physical counter registers from EL0
        const EL0VCTEN = 0x002; // Trap accesses to virtual counter registers from EL0
        const EVNTEN   = 0x004; // Event stream enable
        const EVNTDIR  = 0x008; // 0: 0 to 1 triggers event; 1: 1 to 0 triggers event (see EVNTI)
        const EVNTI    = 0x0F0; // Determines which bit of the counter triggers an event (if event stream enabled)
        const EL0VTEN  = 0x100; // Trap accesses to virtual timer registers from EL0
        const EL0PTEN  = 0x200; // Trap accesses to physical timer registers from EL0
        const RESERVED = 0xffff_fc00;
    }
}

bitflags! {
    /// Bit flags for the Virtual Timer Control Register
    struct CntvCtlEl0: u32 {
        const ENABLE   = 0x1; // Enables the virtual timer
        const IMASK    = 0x2; // If set, the timer won't generate interrupts
        const ISTATUS  = 0x4; // Set automatically when the timer would generate an interrupt, even if masked
        const RESERVED = 0xffff_fff8;
    }
}

lazy_static! {
    unsafe {
        /// The underlying counter's frequency, measured in Hz.
        pub static ref COUNTER_FREQ: Hertz = {
            let cntfrq: u32;

            #[cfg(target_arch = "aarch64")]
            asm!("mrs {:x}, CNTFRQ_EL0", out(reg) cntfrq, options(nomem, nostack, preserves_flags));

            #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
            asm!("mrs {}, CNTFRQ", out(reg) cntfrq, options(nomem, nostack, preserves_flags));

            Hertz(cntfrq)
        };

        /// The counter's precision, measured in femtoseconds.
        pub static ref CLOCK_PRECISION: Femtosecs = hz_to_fs(*COUNTER_FREQ);

        /// The timer used for scheduling threads.
        pub static ref TIMER: Timer = Timer::new().expect("failed to initialize the scheduling timer");
    }
}

impl Timer {
    /// Initializes the Generic Timer as a one-shot timer that can interrupt this CPU.
    pub fn new() -> Result<Self, ()> {
        // Register the IRQ handler if that hasn't been done already.
        let isr = irqs::register_irq(IRQ, on_timer_irq, irqs::Priority::Medium, irqs::IrqTrigger::Edge)?;
        mem::forget(isr);

        Self::init_cntkctl();

        let timer_control = CntvCtlEl0::ENABLE;
        #[cfg(target_arch = "aarch64")]
        unsafe { asm!("msr CNTP_CTL_EL0, {:x}", in(reg) timer_control.bits(), options(nomem, nostack, preserves_flags)); }
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
        unsafe { asm!("msr CNTP_CTL, {}", in(reg) timer_control.bits(), options(nomem, nostack, preserves_flags)); }

        Ok(Timer)
    }

    fn init_cntkctl() {
        let control: u32;
        #[cfg(target_arch = "aarch64")]
        unsafe { asm!("mrs {:x}, CNTKCTL_EL1", out(reg) control, options(nomem, nostack, preserves_flags)); }
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
        unsafe { asm!("mrs {}, CNTKCTL", out(reg) control, options(nomem, nostack, preserves_flags)); }

        // Don't let EL0 code mess with the counter and timer registers, and enable the event
        // stream (important because the interrupts will only go to one CPU).
        // TODO: Is that true? Try looking for "Targeted list model" in the GIC specification.
        let control = CntkctlEl1::from_bits(control).unwrap();
        let control = control & !CntkctlEl1::EL0PCTEN & !CntkctlEl1::EL0VCTEN |
            // Event stream with a period of about 1 millisecond
            CntkctlEl1::EVNTEN | CntkctlEl1::from_bits_truncate((32 - (COUNTER_FREQ.0 / 1000).leading_zeros()) << 16) |
            CntkctlEl1::EL0VTEN | CntkctlEl1::EL0PTEN;

        #[cfg(target_arch = "aarch64")]
        unsafe { asm!("msr CNTKCTL_EL1, {:x}", in(reg) control.bits(), options(nomem, nostack, preserves_flags)); }
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
        unsafe { asm!("msr CNTKCTL, {}", in(reg) control.bits(), options(nomem, nostack, preserves_flags)); }
    }

    /// Causes the timer to issue an interrupt after a duration of at least `delay`, at which point
    /// the given callback will be called.
    pub fn interrupt_after(&self, delay: Duration) {
        let mut new_countdown = ((COUNTER_FREQ.0 as u128) * delay.as_millis() / 1000).try_into().unwrap_or(u32::MAX);

        if new_countdown == 0 {
            // The caller is expecting an interrupt to happen, so we can't just return. Instead,
            // wait a bit longer than requested.
            new_countdown = 1;
        }

        Self::set_countdown(new_countdown);
    }

    #[cfg(target_arch = "aarch64")]
    fn set_countdown(countdown: u32) {
        unsafe { asm!("msr CNTP_TVAL_EL0, {:x}", in(reg) countdown, options(nomem, nostack, preserves_flags)); }
    }
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
    fn set_countdown(countdown: u32) {
        unsafe { asm!("msr CNTP_TVAL, {}", in(reg) countdown, options(nomem, nostack, preserves_flags)); }
    }
}

/// Indicates whether an interrupt has happened since the last time the timer was set. This is needed
/// in case the interrupt arrives while we're in EL1.
#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "Rust" fn scheduling_timer_finished() -> bool {
    // I changed this to always return `false` because the interrupt was always happening after this check.
    /*let timer_control: u32;
    unsafe { asm!("mrs {:x}, CNTP_CTL_EL0", out(reg) timer_control, options(nomem, nostack, preserves_flags)); }
    CntvCtlEl0::from_bits_truncate(timer_control).contains(CntvCtlEl0::ISTATUS)*/
    false
}
/// Indicates whether an interrupt has happened since the last time the timer was set. This is needed
/// in case the interrupt arrives while we're in EL1.
#[no_mangle]
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
pub extern "Rust" fn scheduling_timer_finished() -> bool {
    /*let timer_control: u32;
    unsafe { asm!("mrs {}, CNTP_CTL", out(reg) timer_control, options(nomem, nostack, preserves_flags)); }
    CntvCtlEl0::from_bits_truncate(timer_control).contains(CntvCtlEl0::ISTATUS)*/
    false
}

/// Initializes the Generic Timer as a real-time clock.
pub fn init_clock_per_cpu() -> Result<(), ()> {
    // Nothing to do here. The clock is already running.
    Ok(())
}

/// Gets the amount of time since the counter was at 0, measured in ticks of the counter.
pub fn get_ticks_elapsed() -> u64 {
    let ticks: u64;

    #[cfg(target_arch = "aarch64")]
    unsafe { asm!("mrs {}, CNTPCT_EL0", out(reg) ticks, options(nomem, nostack, preserves_flags)); }
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
    unsafe { asm!("mrs {}, CNTPCT", out(reg) ticks, options(nomem, nostack, preserves_flags)); }

    ticks
}

fn on_timer_irq() -> IsrResult {
    // Make sure the virtual timer actually sent an interrupt.
    let cntv_ctl: u32;

    #[cfg(target_arch = "aarch64")]
    unsafe { asm!("mrs {:x}, CNTP_CTL_EL0", out(reg) cntv_ctl, options(nomem, nostack, preserves_flags)); }
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
    unsafe { asm!("mrs {}, CNTP_CTL", out(reg) cntv_ctl, options(nomem, nostack, preserves_flags)); }

    let cntv_ctl = CntvCtlEl0::from_bits(cntv_ctl).unwrap();
    if cntv_ctl.contains(CntvCtlEl0::IMASK) || !cntv_ctl.contains(CntvCtlEl0::ISTATUS) {
        return IsrResult::WrongIsr;
    }

    // The scheduling timer's only job is to pre-empt the currently running thread and return to the
    // scheduler.
    // FIXME: Somehow keep a record of the IRQ happening if it happens in EL1. In that state, a
    // context switch is impossible (because it would trample the kernel's stack). We just need a
    // single CPU-private bit. A register that Rust is guaranteed never to use could work, but there
    // might not be a good candidate on every architecture.
    //  Aarch64: ESP_EL0[63] (the high bit of EL0's SP should always be 0 in normal operation)
    //  ARM32: ???
    //  X86-64: ??? (Maybe an FPU or SSE register?)
    IsrResult::PreemptThread
}
