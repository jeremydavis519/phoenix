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

//! This crate defines the necessary types for keeping track of time in the Phoenix kernel. It is
//! based on the Rust standard library's `std::time` module.

#![no_std]

#![deny(warnings, missing_docs)]

use {
    core::{
        fmt::{self, Display, Formatter},
        mem,
        ops::*,
        ptr,
        sync::atomic::{AtomicU64, AtomicPtr, Ordering}
    },

    error::Error
};

pub use core::time::Duration;

// TODO: Instead of defining all these unit types, try using the `dimensioned` crate.
/// Represents a number of Hertz.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hertz(pub u32);

/// Represents a number of milliseconds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millisecs(pub u64);

/// Represents a number of nanoseconds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nanosecs(pub u64);

/// Represents a number of femtoseconds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Femtosecs(pub u64);

/// Converts a frequency in Hertz to a period (rounded up) in femtoseconds.
pub const fn hz_to_fs(freq: Hertz) -> Femtosecs {
    Femtosecs((1_000_000_000_000_000 + (freq.0 as u64) - 1) / (freq.0 as u64))
}

macro_rules! impl_arith {
    ( $unit:tt, $base_type:ty ) => {
        impl $unit {
            /// Adds `other` to `self`, wrapping around on overflow.
            pub fn wrapping_add(self, other: $unit) -> $unit {
                $unit(self.0.wrapping_add(other.0))
            }

            /// Subtracts `other` from `self`, wrapping around on underflow.
            pub fn wrapping_sub(self, other: $unit) -> $unit {
                $unit(self.0.wrapping_sub(other.0))
            }
        }

        impl core::ops::Add<$unit> for $unit {
            type Output = $unit;
            
            #[inline]
            fn add(self, other: $unit) -> $unit {
                $unit(self.0 + other.0)
            }
        }
        
        impl core::ops::Sub<$unit> for $unit {
            type Output = $unit;
            
            #[inline]
            fn sub(self, other: $unit) -> $unit {
                $unit(self.0 - other.0)
            }
        }
        
        impl core::ops::AddAssign<$unit> for $unit {
            #[inline]
            fn add_assign(&mut self, other: $unit) {
                self.0 += other.0;
            }
        }
        
        impl core::ops::SubAssign<$unit> for $unit {
            #[inline]
            fn sub_assign(&mut self, other: $unit) {
                self.0 -= other.0;
            }
        }
        
        impl core::ops::Mul<$base_type> for $unit {
            type Output = $unit;
            
            #[inline]
            fn mul(self, other: $base_type) -> $unit {
                $unit(self.0 * other)
            }
        }
        
        impl core::ops::Div<$base_type> for $unit {
            type Output = $unit;
            
            #[inline]
            fn div(self, other: $base_type) -> $unit {
                $unit(self.0 / other)
            }
        }
        
        impl core::ops::Rem<$base_type> for $unit {
            type Output = $unit;
            
            #[inline]
            fn rem(self, other: $base_type) -> $unit {
                $unit(self.0 % other)
            }
        }
        
        impl core::ops::Div<$unit> for $unit {
            type Output = $base_type;
            
            #[inline]
            fn div(self, other: $unit) -> $base_type {
                self.0 / other.0
            }
        }
        
        impl core::ops::Rem<$unit> for $unit {
            type Output = $unit;
            
            #[inline]
            fn rem(self, other: $unit) -> $unit {
                $unit(self.0 % other.0)
            }
        }
        
        impl core::ops::MulAssign<$base_type> for $unit {
            #[inline]
            fn mul_assign(&mut self, other: $base_type) {
                self.0 *= other;
            }
        }
        
        impl core::ops::DivAssign<$base_type> for $unit {
            #[inline]
            fn div_assign(&mut self, other: $base_type) {
                self.0 /= other;
            }
        }
        
        impl core::ops::RemAssign<$base_type> for $unit {
            #[inline]
            fn rem_assign(&mut self, other: $base_type) {
                self.0 %= other;
            }
        }
        
        impl core::ops::RemAssign<$unit> for $unit {
            #[inline]
            fn rem_assign(&mut self, other: $unit) {
                self.0 %= other.0;
            }
        }

        impl From<$unit> for $base_type {
            fn from(x: $unit) -> $base_type {
                x.0
            }
        }

        impl From<$base_type> for $unit {
            fn from(x: $base_type) -> $unit {
                $unit(x)
            }
        }
    };
}

impl_arith!(Hertz, u32);
impl_arith!(Millisecs, u64);
impl_arith!(Nanosecs, u64);
impl_arith!(Femtosecs, u64);

/// A measurement of the system clock. No guarantee is made that this is monotonically increasing.
/// In fact, since the user can manually set the system time, we should expect this to decrease
/// sometimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SystemTime(Nanosecs);

/// An error returned by certain `SystemTime` methods. Indicates how far in the opposite direction a system time lies.
#[derive(Debug, Clone, Copy)]
pub struct SystemTimeError(Nanosecs);

// A function that returns the amount of time that has elapsed since some unknown but constant
// time in the past.
static NOW_RAW: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

// An offset added to the hardware clock's output to get the real current time.
static TIME_OFFSET: AtomicU64 = AtomicU64::new(0);

impl SystemTime {
    /// An anchor in time which can be used to create new `SystemTime` instances or learn about where
    /// in time a `SystemTime` lies.
    pub const UNIX_EPOCH: SystemTime = SystemTime(Nanosecs(0));

    /// Registers a function that returns the number of nanoseconds since some constant time in the
    /// past.
    ///
    /// # Panics
    /// If the function is called more than once with different values.
    pub fn set_now_raw(func: fn() -> Nanosecs) {
        match NOW_RAW.compare_exchange(ptr::null_mut(), func as *mut (), Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {},
            Err(ptr) => assert_eq!(func as *mut (), ptr)
        };
    }

    /// Returns the current system time (equivalent to `SystemTime::now` in the Rust standard library).
    ///
    /// # Panics
    /// If `set_now_raw` has not been called.
    pub fn now() -> SystemTime {
        let counter = NOW_RAW.load(Ordering::Acquire);
        assert!(!counter.is_null());
        let counter: fn() -> Nanosecs = unsafe { mem::transmute(counter) };
        SystemTime::from_raw_nanosecs(counter().0)
    }

    /// Returns the current system time, without the offset applied by `set_now`. In other words,
    /// the epoch used is when the timer started, not the UNIX epoch.
    ///
    /// # Panics
    /// If `set_now_raw` has not been called.
    pub fn now_raw() -> SystemTime {
        let counter = NOW_RAW.load(Ordering::Acquire);
        assert!(!counter.is_null());
        let counter: fn() -> Nanosecs = unsafe { mem::transmute(counter) };
        SystemTime::UNIX_EPOCH + Duration::from_nanos(counter().0)
    }

    /// Returns a `SystemTime` from the given timestamp, which should have been generated by the
    /// system timer. This is a "raw" timestamp in the sense that it comes from the hardware, but
    /// the offset applied by `set_now` is still used (so the epoch used is the UNIX epoch).
    pub fn from_raw_nanosecs(nanosecs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(nanosecs.wrapping_add(TIME_OFFSET.load(Ordering::Acquire)))
    }

    /// Sets the current system time. This is an extension to Rust's standard library, which (being
    /// designed for userspace prorams) doesn't allow anything to change the system time.
    pub fn set_now(time: SystemTime) {
        match time.duration_since(SystemTime::now()) {
            Ok(duration) => TIME_OFFSET.fetch_add(duration.as_nanos() as u64, Ordering::AcqRel),
            Err(e) => TIME_OFFSET.fetch_sub(e.duration().as_nanos() as u64, Ordering::AcqRel)
        };
    }

    /// Returns the amount of time between this `SystemTime` and an earlier one.
    /// If the earlier one is actually later (or at least appears to be later),
    /// a SystemTimeError will be returned instead, which will still indicate the
    /// duration between the two times.
    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
        if self.0 >= earlier.0 {
            let duration_nanos: u64 = (self.0 - earlier.0).0;
            Ok(Duration::new(duration_nanos / 1_000_000_000, (duration_nanos % 1_000_000_000) as u32))
        } else {
            let duration: Nanosecs = earlier.0 - self.0;
            Err(SystemTimeError(duration))
        }
    }

    /// Returns the amount of time between now and the creation of this `SystemTime` (assuming it was
    /// created by calling the `now` method). If this `SystemTime` is actually in the future (or at
    /// least appears to be), a SystemTimeError will be returned instead, which will still indicate
    /// the duration between the two times.
    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(*self)
    }

    // TODO: Maybe add a convenience method for converting a `SystemTime` into a UNIX timestamp?
}

impl Add<Duration> for SystemTime {
    type Output = SystemTime;

    fn add(self, dur: Duration) -> SystemTime {
        SystemTime(self.0 + Nanosecs(dur.as_nanos() as u64))
    }
}

impl AddAssign<Duration> for SystemTime {
    fn add_assign(&mut self, other: Duration) {
        self.0 += Nanosecs(other.as_nanos() as u64);
    }
}

impl Sub<Duration> for SystemTime {
    type Output = SystemTime;

    fn sub(self, dur: Duration) -> SystemTime {
        SystemTime(self.0 - Nanosecs(dur.as_nanos() as u64))
    }
}

impl SubAssign<Duration> for SystemTime {
    fn sub_assign(&mut self, other: Duration) {
        self.0 -= Nanosecs(other.as_nanos() as u64);
    }
}

impl SystemTimeError {
    /// Returns the positive duration which represents how far forward the second system time was from the first.
    pub fn duration(&self) -> Duration {
        Duration::new((self.0).0 / 1_000_000_000, ((self.0).0 % 1_000_000_000) as u32)
    }
}

impl Error for SystemTimeError {}

impl Display for SystemTimeError {
    // TODO: The letters used to abbreviate the time in here ("y", "d", etc.) only make sense in
    // English.
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let mut nanos = self.0;
        let mut only_nanos = true;

        let nanos_per_second = Nanosecs(1_000_000_000);
        let nanos_per_minute = nanos_per_second * 60;
        let nanos_per_hour = nanos_per_minute * 60;
        let nanos_per_day = nanos_per_hour * 24;
        let nanos_per_year = nanos_per_day * 365 + nanos_per_second * 20952; // An average, taking leap years into account

        if nanos >= nanos_per_year {
            if write!(f, "{}y", nanos / nanos_per_year).is_err() {
                return Err(fmt::Error);
            }
            nanos %= nanos_per_year;
            only_nanos = false;
        }
        if nanos >= nanos_per_day {
            if write!(f, "{}d", nanos / nanos_per_day).is_err() {
                return Err(fmt::Error);
            }
            nanos %= nanos_per_day;
            only_nanos = false;
        }
        if nanos >= nanos_per_hour {
            if write!(f, "{}h", nanos / nanos_per_hour).is_err() {
                return Err(fmt::Error);
            }
            nanos %= nanos_per_hour;
            only_nanos = false;
        }
        if nanos >= nanos_per_minute {
            if write!(f, "{}m", nanos / nanos_per_minute).is_err() {
                return Err(fmt::Error);
            }
            nanos %= nanos_per_minute;
            only_nanos = false;
        }
        if nanos >= nanos_per_second {
            if write!(f, "{}s", nanos / nanos_per_second).is_err() {
                return Err(fmt::Error);
            }
            nanos %= nanos_per_second;
            only_nanos = false;
        }
        if nanos > Nanosecs(0) || only_nanos {
            if write!(f, "{}ns", nanos.0).is_err() {
                return Err(fmt::Error);
            }
        }

        Ok(())
    }
}
