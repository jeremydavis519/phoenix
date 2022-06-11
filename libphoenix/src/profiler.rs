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

#[cfg(feature = "profiler")]
use {
    core::{
        cell::Cell,
        ptr,
        sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, AtomicPtr, Ordering}
    }
};

/// Defines a probe that counts every time the calling line is visited (thereby measuring
/// throughput) and returns a reference to it. If an argument is given, that argument must be a
/// reference to a pre-existing probe. In that case, the pair of probes measures latency in addition
/// to throughput.
#[macro_export]
macro_rules! profiler_probe {
    () => {{
        #[cfg(feature = "profiler")] {
            static PROBE: $crate::profiler::Probe = $crate::profiler::Probe::new(
                file!(),
                line!(),
                column!(),
                module_path!()
            );
            PROBE.register(None);
            PROBE.visit();
            &PROBE
        }
        #[cfg(not(feature = "profiler"))] {
            &$crate::profiler::Probe
        }
    }};

    ($prev_probe:expr) => {{
        #[cfg(feature = "profiler")] {
            static PROBE: $crate::profiler::Probe = $crate::profiler::Probe::new(
                file!(),
                line!(),
                column!(),
                module_path!()
            );
            PROBE.register(Some($prev_probe));
            PROBE.visit();
            &PROBE
        }
        #[cfg(not(feature = "profiler"))] {
            $prev_probe;
            &$crate::profiler::Probe
        }
    }};
}

/// Resets the profiler by setting all visit counts to 0.
pub fn reset() {
    #[cfg(feature = "profiler")] {
        for i in 0 .. PROBES_COUNT.load(Ordering::Acquire) {
            let probe_ptr = ALL_PROBES[i].load(Ordering::Acquire);
            if !probe_ptr.is_null() {
                // SAFETY: All probes have static lifetimes. A non-null pointer guarantees a valid
                //         probe.
                unsafe {
                    (*probe_ptr).reset();
                }
            }
        }
    }
}

/// Returns an iterator over all the probes that have been visited so far.
pub fn all_probes() -> impl Iterator<Item = &'static Probe> {
    #[cfg(feature = "profiler")] {
        struct Probes {
            index: usize,
            limit: usize
        }
        impl Iterator for Probes {
            type Item = &'static Probe;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    let i = self.index;
                    if i >= self.limit {
                        return None;
                    }
                    self.index += 1;

                    let ptr = ALL_PROBES[i].load(Ordering::Acquire);
                    if !ptr.is_null() {
                        // SAFETY: All probes have static lifetimes. A non-null pointer guarantees a
                        //         valid probe.
                        return Some(unsafe { &*ptr });
                    }
                }
            }
        }
        Probes {
            index: 0,
            limit: PROBES_COUNT.load(Ordering::Acquire)
        }
    }
    #[cfg(not(feature = "profiler"))] {
        [].iter()
    }
}


#[cfg(feature = "profiler")]
const MAX_PROBES: usize = 1000;

#[cfg(feature = "profiler")]
static ALL_PROBES: [AtomicPtr<Probe>; MAX_PROBES] = [const { AtomicPtr::new(ptr::null_mut()) }; MAX_PROBES];

#[cfg(feature = "profiler")]
static PROBES_COUNT: AtomicUsize = AtomicUsize::new(0);

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

/// A probe used by the profiler to measure performance. Use the [`profiler_probe`] macro to
/// construct one.
#[cfg(feature = "profiler")]
#[derive(Debug)]
pub struct Probe {
    file:                            &'static str,
    line:                            u32,
    column:                          u32,
    module:                          &'static str,
    // TODO: Also record the function name. This seems to require a procedural macro.
    prev_probe:                      Cell<Option<&'static Probe>>,
    visits:                          AtomicU64,
    current_visitors:                AtomicU64,
    last_reset_time_nanos:           AtomicU64,
    last_visitors_change_time_nanos: AtomicU64,
    total_visitor_nanos:             AtomicU64,
    registered:                      AtomicBool
}
#[cfg(not(feature = "profiler"))]
#[derive(Debug)]
#[doc(hidden)]
pub struct Probe;

impl Probe {
    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub const fn new(
        file:       &'static str,
        line:       u32,
        column:     u32,
        module:     &'static str
    ) -> Self {
        Self {
            file,
            line,
            column,
            module,
            prev_probe:                      Cell::new(None),
            visits:                          AtomicU64::new(0),
            current_visitors:                AtomicU64::new(0),
            last_reset_time_nanos:           AtomicU64::new(0),
            last_visitors_change_time_nanos: AtomicU64::new(0),
            total_visitor_nanos:             AtomicU64::new(0),
            registered:                      AtomicBool::new(false)
        }
    }

    /// The file in which the probe is defined.
    pub const fn file(&self) -> &'static str {
        #[cfg(feature = "profiler")] { self.file }
        #[cfg(not(feature = "profiler"))] { "" }
    }

    /// The line number on which the probe is defined.
    pub const fn line(&self) -> u32 {
        #[cfg(feature = "profiler")] { self.line }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The column number on which the probe is defined.
    pub const fn column(&self) -> u32 {
        #[cfg(feature = "profiler")] { self.column }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The module in which the probe is defined.
    pub const fn module(&self) -> &'static str {
        #[cfg(feature = "profiler")] { self.module }
        #[cfg(not(feature = "profiler"))] { "" }
    }

    /// The probe immediately before this one, if any.
    pub fn prev_probe(&self) -> Option<&'static Probe> {
        #[cfg(feature = "profiler")] { self.prev_probe.get() }
        #[cfg(not(feature = "profiler"))] { None }
    }

    /// The number of visits recorded since the last reset.
    pub fn visits(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.visits.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    /// The number of visitors that have reached this probe but have not yet reached the next probe
    /// (i.e. a probe whose `prev_probe` is this one).
    pub fn current_visitors(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.current_visitors.load(Ordering::Acquire) }
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
        #[cfg(feature = "profiler")] { self.last_reset_time_nanos.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    // A timestamp from the last time execution entered or exited this probe's section of code.
    #[cfg(feature = "profiler")]
    fn last_visitors_change_time_nanos(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.last_visitors_change_time_nanos.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    // A running total of concurrent visitors times the amount of time spent between changes. This
    // is used to calculate the average number of concurrent visitors, which is necessary for
    // measuring average latency.
    #[cfg(feature = "profiler")]
    fn total_visitor_nanos(&self) -> u64 {
        #[cfg(feature = "profiler")] { self.total_visitor_nanos.load(Ordering::Acquire) }
        #[cfg(not(feature = "profiler"))] { 0 }
    }

    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub fn register(&'static self, prev_probe: Option<&'static Probe>) {
        if self.registered.swap(true, Ordering::AcqRel) {
            // Already registered.
            return;
        }

        // Link to the earlier probe, if any.
        self.prev_probe.set(prev_probe);

        // This is the first time we're visiting the probe.
        let now = current_time_nanos();
        self.last_reset_time_nanos.store(now, Ordering::Release);
        self.last_visitors_change_time_nanos.store(now, Ordering::Release);

        // Register this probe.
        let idx = PROBES_COUNT.fetch_add(1, Ordering::AcqRel);
        assert!(idx < MAX_PROBES, "too many profiler probes");
        ALL_PROBES[idx].store(self as *const Probe as *mut Probe, Ordering::Release);
    }

    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub fn visit(&self) {
        self.visits.fetch_add(1, Ordering::AcqRel);

        // Record a visitor entering this section.
        let now = current_time_nanos();
        let last_visit_time = self.last_visitors_change_time_nanos.swap(now, Ordering::AcqRel);
        let old_visitors = self.current_visitors.fetch_add(1, Ordering::AcqRel);
        self.total_visitor_nanos.fetch_add(
            (old_visitors as i64).wrapping_mul(now.wrapping_sub(last_visit_time) as i64) as u64,
            Ordering::AcqRel
        );

        if let Some(prev_probe) = self.prev_probe.get() {
            // Record a visitor leaving the previous section.
            let now = current_time_nanos();
            let last_visit_time = prev_probe.last_visitors_change_time_nanos.swap(now, Ordering::AcqRel);
            let old_visitors = prev_probe.current_visitors.fetch_sub(1, Ordering::AcqRel);
            prev_probe.total_visitor_nanos.fetch_add(
                (old_visitors as i64).wrapping_mul(now.wrapping_sub(last_visit_time) as i64) as u64,
                Ordering::AcqRel
            );
        }
    }

    #[cfg(feature = "profiler")]
    fn reset(&self) {
        self.visits.store(0, Ordering::Release);
        self.current_visitors.store(0, Ordering::Release);
        self.last_visitors_change_time_nanos.store(current_time_nanos(), Ordering::Release);
        self.total_visitor_nanos.store(0, Ordering::Release);
    }
}

unsafe impl Sync for Probe {}
