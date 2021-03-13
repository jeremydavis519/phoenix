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

//! This crate provides a platform-independent interface to the timer devices that the kernel uses
//! for scheduling, keeping track of real-world time, and anything else it might need a timer for.

#![no_std]
#![feature(asm)]

#![deny(warnings, missing_docs)]

#[macro_use] extern crate cfg_if;

use time::{Nanosecs, SystemTime};

cfg_if! {
    if #[cfg(target_machine = "qemu-virt")] {
        #[macro_use] extern crate bitflags;
        #[macro_use] extern crate shared;

        #[cfg(feature = "self-test")] use shared::wait_for_event;

        pub mod arm;
        use self::arm::generic_timer as scheduling_timer;
        use self::arm::pl031 as realtime_clock;
    } else if #[cfg(target_arch = "x86_64")] {
        #[allow(missing_docs)]
        mod scheduling_timer {
            use {time::Duration, crate::Timer};
            #[cfg(feature = "self-test")]
            pub static TIMER_WORKS: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
            pub static TIMER: &Timer = &Timer;
            impl Timer {
                pub fn interrupt_after(&self, _duration: Duration) { unimplemented!() }
            }
            pub extern "Rust" fn scheduling_timer_finished() -> bool { unimplemented!() }
        }
        #[allow(missing_docs)]
        mod realtime_clock {
            use time::{Hertz, Femtosecs};
            pub static COUNTER_FREQ: Hertz = Hertz(1);
            pub static CLOCK_PRECISION: Femtosecs = Femtosecs(1);
            pub fn init_clock_per_cpu() -> Result<(), ()> { Err(()) }
            pub fn get_ticks_elapsed() -> u64 { 0 }
        }
    }
}

/// Represents a one-shot timer, useful for things like schedulers.
#[derive(Debug)]
pub struct Timer;

/// The timer used for scheduling threads. This is exposed as a single static variable, but it
/// behaves as if every CPU had a separate timer. On some systems, this is actually the case. On
/// others, the software implementation ensures that the scheduler code never needs to worry about
/// timer conflicts with other CPUs.
pub use self::scheduling_timer::TIMER as SCHEDULING_TIMER;

pub use self::scheduling_timer::scheduling_timer_finished;

/// The system clock's resolution.
pub use self::realtime_clock::CLOCK_PRECISION;

/// Initializes the platform's time-related functions, such as starting the timers.
/// This should be called once by every CPU core.
pub fn init_per_cpu() {
    SystemTime::set_now_raw(get_time_elapsed);

    //scheduling_timer::init_timer_per_cpu().expect("failed to initialize the scheduling timer");
    realtime_clock::init_clock_per_cpu().expect("failed to initialize the real-time clock");

    #[cfg(feature = "self-test")] {
        // Make sure the timers are actually working by waiting for them to tick.
        let initial_time = get_time_elapsed();
        // TODO: print!("Making sure we're receiving events...");
        wait_for_event();
        // TODO: println!(" yes");
        /*// TODO: print!("Waiting to see if the scheduling timer works once...");
        while !scheduling_timer::TIMER_WORKS.load(Ordering::Acquire) {
            wait_for_event();
        }
        // TODO: println!(" yes");
        // TODO: print!("Waiting for the scheduling timer to work again...");
        scheduling_timer::TIMER_WORKS.store(false, Ordering::Release);
        while !scheduling_timer::TIMER_WORKS.load(Ordering::Acquire) {
            wait_for_event();
        }
        // TODO: println!(" yes");*/
        // TODO: print!("Waiting to see if the real-time clock is working...");
        while get_time_elapsed() == initial_time {
            wait_for_event();
        }
        // TODO: println!(" yes");
    }
}

// Gets the amount of time elapsed since some constant, undefined time in the past.
fn get_time_elapsed() -> Nanosecs {
    let ticks = realtime_clock::get_ticks_elapsed();
    Nanosecs((ticks as u128 * 1_000_000_000 as u128 / realtime_clock::COUNTER_FREQ.0 as u128) as u64)
}
