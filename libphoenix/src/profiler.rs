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
            static PROBE: $crate::profiler::Probe = $crate::profiler::Probe{
                file:       file!(),
                line:       line!(),
                column:     column!(),
                module:     module_path!(),
                prev_probe: core::cell::Cell::new(None),
                visits:     core::sync::atomic::AtomicU64::new(0),
                registered: core::sync::atomic::AtomicBool::new(false)
            };
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
            static PROBE: $crate::profiler::Probe = $crate::profiler::Probe {
                file:       file!(),
                line:       line!(),
                column:     column!(),
                module:     module_path!(),
                prev_probe: core::cell::Cell::new(None),
                visits:     core::sync::atomic::AtomicU64::new(0),
                registered: core::sync::atomic::AtomicBool::new(false)
            };
            PROBE.register(Some($prev_probe));
            PROBE.visit();
            &PROBE
        }
        #[cfg(not(feature = "profiler"))] {
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
                    (*probe_ptr).visits.store(0, Ordering::Release);
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

/// A probe used by the profiler to measure performance. Use the [`profiler_probe`] macro to
/// construct one.
#[cfg(feature = "profiler")]
#[derive(Debug)]
pub struct Probe {
    #[doc(hidden)] pub file:       &'static str,
    #[doc(hidden)] pub line:       u32,
    #[doc(hidden)] pub column:     u32,
    #[doc(hidden)] pub module:     &'static str,
    // TODO: Also record the function name. This seems to require a procedural macro.
    #[doc(hidden)] pub prev_probe: Cell<Option<&'static Probe>>,
    #[doc(hidden)] pub visits:     AtomicU64,
    #[doc(hidden)] pub registered: AtomicBool
}
#[cfg(not(feature = "profiler"))]
#[derive(Debug)]
#[doc(hidden)]
pub struct Probe;

impl Probe {
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

    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub fn register(&'static self, prev_probe: Option<&'static Probe>) {
        if self.registered.swap(true, Ordering::AcqRel) {
            // Already registered.
            return;
        }

        // Link to the earlier probe, if any.
        self.prev_probe.set(prev_probe);

        // Register this probe.
        let idx = PROBES_COUNT.fetch_add(1, Ordering::AcqRel);
        assert!(idx < MAX_PROBES, "too many profiler probes");
        ALL_PROBES[idx].store(self as *const Probe as *mut Probe, Ordering::Release);
    }

    #[cfg(feature = "profiler")]
    #[doc(hidden)]
    pub fn visit(&self) {
        self.visits.fetch_add(1, Ordering::AcqRel);
    }
}

unsafe impl Sync for Probe {}
