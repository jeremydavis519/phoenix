/* Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module provides facilities for causal profiling. This is different from traditional profiling
//! in that it can experiment to see where optimization would have the biggest impact on overall
//! performance (i.e. on runtime, latency, or throughput). The idea for such a profiler, and the core
//! insight that makes it possible, came from [Coz](https://github.com/plasma-umass/coz), a standalone
//! causal profiler for userspace programs. The main benefit of implementing one as part of Phoenix is
//! that the kernel can benefit from it too.
//!
//! TODO: Once the API stabilizes, provide examples of how to use this.
//!
//! An effort has been made to minimize the overhead associated with using this profiler, but it is not
//! zero-cost. Having progress points in the program, even if they are not used, will result in a small
//! performance penalty. Therefore, the functionality is locked behind the `profiler` feature. If that
//! feature is not used, the API is still available but does nothing and can be optimized away.

use {
    core::{
        ffi::c_void,
        str
    }
};

#[cfg(feature = "profiler")]
use {
    core::{
        mem,
        ptr,
        slice,
        sync::atomic::{AtomicU64, Ordering}
    }
};

#[cfg(all(feature = "profiler", not(feature = "kernelspace")))]
#[cfg(target_arch = "aarch64")] // FIXME: Remove this condition.
use crate::syscall;

#[cfg(feature = "profiler")]
extern {
    static __profile_probes_start: c_void;
    static __profile_probes_end:   c_void;
}

/// Does some setup to prepare for defining one or more probes in the same file. This macro defines
/// items rather than running code, so it should be invoked outside of function scope.
#[macro_export]
macro_rules! profiler_setup {
    () => {
        const FILENAME_LEN: usize = ::core::file!().len();

        #[cfg(feature = "profiler")]
        #[link_section = ".profile.strings"]
        #[export_name = concat!("profile: ", ::core::file!())]
        static FILENAME: $crate::profiler::ProbeFilename<FILENAME_LEN> =
            $crate::profiler::ProbeFilename::<FILENAME_LEN>::new(::core::file!());
    }
}

/// Defines a probe that counts every time the calling line is visited (thereby measuring
/// throughput). If the a previous probe's identifier is given, the pair of probes measures
/// latency in addition to throughput.
///
/// This macro requires [`profiler_setup`] to be called exactly once in the same file.
#[macro_export]
macro_rules! profiler_probe {
    (@static) => {{
        #[cfg(feature = "profiler")] {
            extern {
                #[link_name = concat!("profile: ", ::core::file!())]
                static FILENAME: $crate::profiler::ProbeFilename<0>;
            }

            #[link_section = ".profile"]
            static PROBE: $crate::profiler::Probe = $crate::profiler::Probe::new(
                unsafe { &FILENAME as *const _ },
                ::core::line!(),
                ::core::column!(),
                None
            );
            &PROBE
        }
        #[cfg(not(feature = "profiler"))] {
            &$crate::profiler::Probe
        }
    }};

    (@static $prev_probe:expr) => {{
        #[cfg(feature = "profiler")] {
            extern {
                #[link_name = concat!("profile: ", ::core::file!())]
                static FILENAME: $crate::profiler::ProbeFilename<0>;
            }

            #[link_section = ".profile"]
            static PROBE: $crate::profiler::Probe = $crate::profiler::Probe::new(
                unsafe { &FILENAME as *const _ },
                ::core::line!(),
                ::core::column!(),
                Some($prev_probe.probe)
            );
            &PROBE
        }
        #[cfg(not(feature = "profiler"))] {
            $prev_probe;
            &$crate::profiler::Probe
        }
    }};

    ($($prev_probe:expr)?) => {
        static PROBE: &$crate::profiler::Probe = $crate::profiler_probe!(@static $($prev_probe)?);
        PROBE.visit();
    };

    ($($prev_probe:expr)? => $name:ident) => {
        static $name: $crate::profiler::ProbeHandle = $crate::profiler::ProbeHandle {
            probe: $crate::profiler_probe!(@static $($prev_probe)?)
        };
        $name.probe.visit();
    }
}

/// An opaque handle returned by [`profiler_probe!`], which can also be passed back to that macro
/// to represent the previous probe.
pub struct ProbeHandle {
    #[doc(hidden)]
    pub probe: &'static Probe
}

/// Resets the profiler by setting all visit counts to 0.
pub fn reset() {
    #[cfg(feature = "profiler")] {
        for probe in probes() {
            probe.reset();
        }
    }
}

/// Returns an iterator over all the probes that have been visited in this process so far.
pub fn probes<'a>() -> impl Iterator<Item = ProbeRef<'a>> + Clone {
    #[cfg(feature = "profiler")] {
        let base_ptr = unsafe { &__profile_probes_start as *const _ };
        let probes = PROBES_HEADER.probes(base_ptr);
        let len = probes.len();
        (0 .. len).map(move |idx| ProbeRef { probes, idx, base_ptr })
    }
    #[cfg(not(feature = "profiler"))] {
        (0 .. 0).map(|_| unreachable!())
    }
}

/// Returns an iterator over all the probes in the kernel's profile that have been visited so far.
#[cfg(not(feature = "kernelspace"))]
pub async fn kernel_probes<'a>() -> impl Iterator<Item = ProbeRef<'a>> + Clone {
    #[cfg(feature = "profiler")] {
        let profile_start;
        #[cfg(not(target_arch = "aarch64"))] { // TODO: Remove this.
            profile_start = unimplemented!();
        }
        #[cfg(target_arch = "aarch64")] { // TODO: Make this unconditional.
            profile_start = syscall::time_view_kernel_profile().await;
        }
        let header = unsafe { &*(profile_start as *const ProbesHeader) };
        let probes_start = (profile_start + mem::size_of::<ProbesHeader>() + 15) / 16 * 16;

        let probes = header.probes(probes_start as *const _);
        let len = probes.len();
        (0 .. len).map(move |idx| ProbeRef { probes, idx, base_ptr: header.probes_start })

        // TODO: Return an object that acts as an iterator but also, when dropped, asks the kernel to unmap the profile.
    }
    #[cfg(not(feature = "profiler"))] {
        (0 .. 0).map(|_| unreachable!())
    }
}


#[cfg(feature = "profiler")]
#[cfg(target_machine = "qemu-virt")]
fn current_time_nanos() -> u64 {
    #[cfg(feature = "kernelspace")] {
        unsafe { current_time_nanos_extern() }
    }
    #[cfg(not(feature = "kernelspace"))] {
        crate::syscall::time_now_unix_nanos()
    }
}

// TODO: Remove this. It's only here to make building on an x86-64 host possible.
#[cfg(feature = "profiler")]
#[cfg(not(target_machine = "qemu-virt"))]
fn current_time_nanos() -> u64 { unimplemented!() }

// We can avoid using a system call to get the current time if we're already running in the kernel.
#[cfg(all(feature = "profiler", feature = "kernelspace", target_machine = "qemu-virt"))]
extern "Rust" {
    fn current_time_nanos_extern() -> u64;
}

#[cfg(feature = "profiler")]
#[link_section = ".profile.header"]
static PROBES_HEADER: ProbesHeader = ProbesHeader {
    _version:     0x0000,
    probes_start: unsafe { &__profile_probes_start as *const _ },
    probes_end:   unsafe { &__profile_probes_end as *const _ }
};

#[repr(C)]
struct ProbesHeader {
    _version: u16,

    // SAFETY: These pointers are not guaranteed to be valid in the usual sense. For instance, we
    // may be in userspace, and these may refer to addresses in kernelspace. But the offset between
    // them is guaranteed to be correct.
    probes_start: *const c_void,
    probes_end:   *const c_void
}

impl ProbesHeader {
    #[cfg(feature = "profiler")]
    fn probes(&self, probes_start: *const c_void) -> &[Probe] {
        let len = (self.probes_end as usize - self.probes_start as usize) / mem::size_of::<Probe>();
        unsafe { slice::from_raw_parts(probes_start as *const Probe, len) }
    }
}

unsafe impl Sync for ProbesHeader {}

#[repr(C)]
#[derive(Debug)]
#[doc(hidden)]
pub struct ProbeFilename<const N: usize> {
    len:   u16,
    bytes: [u8; N]
}

impl<const N: usize> ProbeFilename<N> {
    #[doc(hidden)]
    pub const fn new(filename: &str) -> Self {
        assert!(filename.len() == N);
        assert!(filename.len() <= u16::max_value() as usize);

        let mut pf = ProbeFilename {
            len:   filename.len() as u16,
            bytes: [0; N]
        };
        let bytes = filename.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            pf.bytes[i] = bytes[i];
            i += 1;
        }
        pf
    }

    #[cfg(feature = "profiler")]
    const fn as_str(&self) -> &str {
        // SAFETY: The only way to generate a `ProbeFilename` is from a valid `&str`, so we will always
        // have the right length and valid UTF-8.
        unsafe {
            let bytes = slice::from_raw_parts(
                &self.bytes as *const _ as *const u8,
                self.len as usize
            );
            str::from_utf8_unchecked(bytes)
        }
    }
}

#[cfg(feature = "profiler")]
#[repr(C)]
#[derive(Debug)]
#[doc(hidden)]
pub struct Probe {
    // All pointers in here may be invalid except in terms of their offsets from each other.
    file:                            *const ProbeFilename<0>,
    line:                            u32,
    column:                          u32,
    visits:                          AtomicU64,
    current_visitors:                AtomicU64,
    last_reset_time_nanos:           AtomicU64,
    last_visitors_change_time_nanos: AtomicU64,
    total_visitor_nanos:             AtomicU64,
    prev_probe:                      *const Probe
}
#[cfg(not(feature = "profiler"))]
#[derive(Debug)]
#[doc(hidden)]
pub struct Probe;

impl Probe {
    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub const fn new(
        file:       *const ProbeFilename<0>,
        line:       u32,
        column:     u32,
        prev_probe: Option<&'static Probe>
    ) -> Self {
        let prev_probe = match prev_probe {
            Some(p) => p as *const Probe,
            None => ptr::null()
        };

        Self {
            file,
            line,
            column,
            visits:                          AtomicU64::new(0),
            current_visitors:                AtomicU64::new(0),
            last_reset_time_nanos:           AtomicU64::new(0),
            last_visitors_change_time_nanos: AtomicU64::new(0),
            total_visitor_nanos:             AtomicU64::new(0),
            prev_probe
        }
    }

    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub fn visit(&self) {
        // Record a visitor entering this section.
        self.visits.fetch_add(1, Ordering::AcqRel);
        let now = current_time_nanos();
        let last_visit_time = self.last_visitors_change_time_nanos.swap(now, Ordering::AcqRel);
        let old_visitors = self.current_visitors.fetch_add(1, Ordering::AcqRel);
        self.total_visitor_nanos.fetch_add(
            (old_visitors as i64).wrapping_mul(now.wrapping_sub(last_visit_time) as i64) as u64,
            Ordering::AcqRel
        );

        if !self.prev_probe.is_null() {
            // Record a visitor leaving the previous section.
            let prev_probe = unsafe { &*self.prev_probe };
            let now = current_time_nanos();
            let last_visit_time = prev_probe.last_visitors_change_time_nanos.swap(now, Ordering::AcqRel);
            let old_visitors = prev_probe.current_visitors.fetch_sub(1, Ordering::AcqRel);
            prev_probe.total_visitor_nanos.fetch_add(
                (old_visitors as i64).wrapping_mul(now.wrapping_sub(last_visit_time) as i64) as u64,
                Ordering::AcqRel
            );
        }
    }
}

/// A reference to a probe used by the profiler to measure performance. Use the [`profiler_probe`]
/// macro to construct a probe and the [`probes`] function to access them.
#[repr(C)]
#[derive(Debug)]
pub struct ProbeRef<'a> {
    probes:   &'a [Probe],
    idx:      usize,

    // A pointer, possibly into another address space, by which other pointers can be measured.
    base_ptr: *const c_void
}

impl<'a> ProbeRef<'a> {
    #[cfg(feature = "profiler")]
    const fn probe(&self) -> &Probe {
        &self.probes[self.idx]
    }

    /// The file in which the probe is defined.
    pub fn file(&self) -> &str {
        #[cfg(feature = "profiler")] {
            let base = self.probes as *const _ as *const u8;
            let offset = self.probe().file as usize - self.base_ptr as usize;
            unsafe { (*(base.add(offset) as *const ProbeFilename<0>)).as_str() }
        }
        #[cfg(not(feature = "profiler"))] { "" }
    }

    /// The line number on which the probe is defined.
    pub const fn line(&self) -> u32 {
        #[cfg(feature = "profiler")] { self.probe().line }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The column number on which the probe is defined.
    pub const fn column(&self) -> u32 {
        #[cfg(feature = "profiler")] { self.probe().column }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The probe immediately before this one, if any.
    pub fn prev_probe(&self) -> Option<ProbeRef<'a>> {
        #[cfg(feature = "profiler")] {
            let ptr = self.probe().prev_probe;
            if ptr.is_null() {
                None
            } else {
                let idx = unsafe { ptr.offset_from(self.base_ptr as *const Probe) } as usize;
                Some(Self { probes: self.probes, idx: idx, base_ptr: self.base_ptr })
            }
        }
        #[cfg(not(feature = "profiler"))] { None }
    }

    /// The number of visits recorded since the last reset.
    pub fn visits(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.probe().visits.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The number of visitors that have reached this probe but have not yet reached the next probe
    /// (i.e. a probe whose `prev_probe` is this one).
    pub fn current_visitors(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.probe().current_visitors.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The average throughput this probe has measured since its last reset, measured in Hertz.
    pub fn avg_throughput_hz(&self) -> f64 {
        #[cfg(feature = "profiler")] {
            self.avg_throughput_hz_impl(self.visits(), current_time_nanos(), self.last_reset_time_nanos())
        }
        #[cfg(not(feature = "profiler"))] { 0.0 }
    }

    #[cfg(feature = "profiler")]
    fn avg_throughput_hz_impl(&self, visits: u64, now_nanos: u64, reset_time_nanos: u64) -> f64 {
        visits as f64 / now_nanos.wrapping_sub(reset_time_nanos) as i64 as f64 * 1_000_000_000.0
    }

    /// The average number of visitors in this probe's section of code since the last reset.
    pub fn avg_visitors(&self) -> f64 {
        #[cfg(feature = "profiler")] {
            self.avg_visitors_impl(current_time_nanos(), self.last_reset_time_nanos())
        }
        #[cfg(not(feature = "profiler"))] { 0.0 }
    }

    #[cfg(feature = "profiler")]
    fn avg_visitors_impl(&self, now_nanos: u64, reset_time_nanos: u64) -> f64 {
        let total_visitor_nanos = self.total_visitor_nanos()
            .wrapping_add(
                (self.current_visitors() as i64)
                    .wrapping_mul(now_nanos.wrapping_sub(self.last_visitors_change_time_nanos()) as i64)
                    as u64
            );
        total_visitor_nanos as f64 / now_nanos.wrapping_sub(reset_time_nanos) as i64 as f64
    }

    /// The average latency this probe has measured since its last reset, measured in seconds.
    pub fn avg_latency_secs(&self) -> Option<f64> {
        #[cfg(feature = "profiler")] {
            self.avg_latency_secs_impl(self.visits(), current_time_nanos(), self.last_reset_time_nanos())
        }
        #[cfg(not(feature = "profiler"))] { None }
    }

    #[cfg(feature = "profiler")]
    fn avg_latency_secs_impl(&self, visits: u64, now_nanos: u64, reset_time_nanos: u64) -> Option<f64> {
        if let Some(prev_probe) = self.prev_probe() {
            let throughput = self.avg_throughput_hz_impl(visits, now_nanos, reset_time_nanos);
            if throughput != 0.0 {
                Some(prev_probe.avg_visitors_impl(now_nanos, reset_time_nanos) / throughput)
            } else {
                None
            }
        } else {
            None
        }
    }

    // A timestamp from the last time `reset` was called (or when the probe was created).
    #[cfg(feature = "profiler")]
    fn last_reset_time_nanos(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.probe().last_reset_time_nanos.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    // A timestamp from the last time execution entered or exited this probe's section of code.
    #[cfg(feature = "profiler")]
    fn last_visitors_change_time_nanos(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.probe().last_visitors_change_time_nanos.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    // A running total of concurrent visitors times the amount of time spent between changes. This
    // is used to calculate the average number of concurrent visitors, which is necessary for
    // measuring average latency.
    #[cfg(feature = "profiler")]
    fn total_visitor_nanos(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.probe().total_visitor_nanos.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    #[cfg(feature = "profiler")]
    fn reset(&self) {
        let probe = self.probe();
        probe.visits.store(0, Ordering::Release);
        probe.current_visitors.store(0, Ordering::Release);
        probe.last_visitors_change_time_nanos.store(current_time_nanos(), Ordering::Release);
        probe.total_visitor_nanos.store(0, Ordering::Release);
        probe.last_reset_time_nanos.store(current_time_nanos(), Ordering::Release);
    }
}

unsafe impl Sync for Probe {}
