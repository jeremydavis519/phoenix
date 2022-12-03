/* Copyright (c) 2018-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! Phoenix's scheduler. It is designed as a simple round-robin scheduler, with each thread
//! executed on a first-come, first-served basis. Threads do have priorities, however. Instead of
//! preempting other threads, high-priority threads simply have longer quanta--that is, they run
//! for longer periods of time.
//!
//! For the sake of efficiency in the scheduler, the exact order of threads is allowed to change in
//! unspecified ways at unspecified times, but not in a way that could lead to a thread's
//! starvation. More precisely, the scheduler works through the list of threads in laps, and while
//! the exact order of the upcoming threads is unspecified, every thread will be run exactly once
//! per lap (unless it is sleeping, otherwise blocked, or being moved between CPUs for load
//! balancing).

#![no_std]

#![deny(warnings, missing_docs)]

#![cfg_attr(target_arch = "aarch64", feature(allocator_api))]

extern crate alloc;

#[macro_use] extern crate shared;

use {
    alloc::sync::Arc,
    core::{
        sync::atomic::{AtomicU32, AtomicUsize, Ordering},
        time::Duration,
    },
    collections::{AtomicLinkedList, AtomicLinkedListSemaphore},
    io::printlndebug,
    locks::Semaphore,
    shared::{/*count_cpus, cpu_index,*/ wait_for_event},
    fs::File,
    timers::SCHEDULING_TIMER,
};

pub mod process;
mod thread;

pub use {
    process::Process,
    thread::{Thread, ThreadStatus, ThreadQueue, ThreadCreationError},
};

const fn quantum(priority: u8) -> Duration {
    Duration::from_millis(priority as u64)
}

/// Uses the given thread queue to run threads forever (or until the computer is turned off).
pub fn run(mut thread_queue: ThreadQueue<File>) -> ! {
    // TODO: let cpu_index = cpu_index();
    // TODO: let cpu_count = count_cpus();
    // FIXME: These definitions need to be replaced by the function calls shown above.
    let cpu_index = 0;
    let cpu_count = 1;

    let mut priority_sum = thread_queue.iter().fold(0, |s, thread| s + u32::from(thread.priority()));

    // This is our pseudorandom number generator for load balancing. It is _extremely_ simple, for
    // two reasons: (1) we want to spend as little time as possible between threads (and constant
    // time if possible), and (2) we don't really need any unpredictability at all. A counter
    // combined with how the threads are shifted around in the queue at random times in response to
    // threads blocking, sleeping, or being terminated should be plenty. We use multiple streams
    // with different step sizes to remove long-term correlations between related uses of the PRNG.
    // The `rand_state` values were taken from some PCG output. The `rand_step` values need to be
    // coprime with both 2^64 and each other to avoid correlations, so I picked some prime numbers.
    const PRNG_PICK_THREAD:    usize = 0;
    const PRNG_FUZZY_PRIORITY: usize = 1;
    let n = cpu_index as u64;
    let mut rand_state: [u64; 8] = [n * 0x7e1c, n * 0x0330, n * 0x0899, n * 0x0e8e,
        n * 0xc5f8, n * 0xffa8, n * 0x98b9, n * 0x2b24];
    let rand_step: [u64; 8] = [1223, 2731, 4391, 6113, 7879, 9679, 11587, 13441];
    let mut rand = move |stream: usize| {
        rand_state[stream] = rand_state[stream].wrapping_add(rand_step[stream]);
        rand_state[stream]
    };
    drop(n);

    loop {
        // Run all the threads we currently have.
        let mut i = 0;
        while i < thread_queue.len() {
            SCHEDULING_TIMER.interrupt_after(quantum(thread_queue[i].priority()));
            match thread_queue[i].run() {
                ThreadStatus::Running => {
                    // Just move on to the next thread.
                    i += 1;
                },
                ThreadStatus::Sleeping => {
                    // TODO
                    unimplemented!("putting a thread to sleep");
                },
                ThreadStatus::Terminated => {
                    // Remove the terminated thread.
                    printlndebug!("Terminating thread {:#x}", thread_queue[i].id());
                    priority_sum -= u32::from(thread_queue.swap_remove(i).priority());
                },
            };
        }

        // The rest of this loop should have a time complexity of O(1) in order to get back to the
        // running threads as soon as possible.

        // Load balancing:
        if let Ok(moving_threads) = MOVING_THREADS.try_access() {
            // Each CPU will try to keep the sum of its threads' priorities (which is proportional to
            // the amount of time needed to schedule each of them once) around this level, although
            // some pseudorandom jitter is used to move threads around in ways that will improve
            // efficiency overall.
            let ideal_priority_sum = (TOTAL_PRIORITY.load(Ordering::Acquire) + cpu_index) / cpu_count;

            // We do things even if we're at the ideal priority sum because there might be a better
            // distribution of threads in terms of affinity.
            if priority_sum <= ideal_priority_sum {
                // We can afford to take another thread. We don't randomize this choice because it's
                // best not to leave any threads in the "moving" list for too long. Running them
                // suboptimally is better than not at all.
                if let Some(thread) = moving_threads.head() {
                    if let Ok(thread) = moving_threads.remove_head(thread) {
                        priority_sum += u32::from(thread.priority());
                        thread_queue.push(thread);
                    }
                }
            }
            if cpu_count > 1 && priority_sum >= ideal_priority_sum && thread_queue.len() > 1 {
                // We have more threads than we need.
                let idx = rand(PRNG_PICK_THREAD) as usize % thread_queue.len();
                let thread = &thread_queue[idx];
                if priority_sum - u32::from(thread.priority()) >= ideal_priority_sum - rand(PRNG_FUZZY_PRIORITY) as u32 & u32::from(u8::MAX) {
                    // The thread's priority is low enough that we'll stay near the ideal sum if we
                    // remove it.
                    // TODO: Do a randomized CPU affinity check, weighted by the ratio (in the
                    // thread's process) of threads on this CPU to threads elsewhere, before
                    // deciding to remove the thread. The goal is to run threads in the same
                    // address space on the same CPU to minimize TLB misses.
                    let thread = thread_queue.swap_remove(idx);
                    match moving_threads.insert_head(thread) {
                        Ok(()) => {}, // Thread successfully moved
                        Err(thread) => thread_queue.push(thread) // Thread couldn't be moved right now
                    };
                }
            }
        }

        // If this CPU has no threads, wait for a bit before seeing if one becomes available.
        if thread_queue.is_empty() {
            wait_for_event();
        }
    }
}

/// Spawns a new thread that will begin execution as soon as possible.
///
/// # Returns
/// The thread's ID, which a userspace application can use to refer to it when making system calls.
pub fn spawn_thread(
        process:        Arc<Process<File>>,
        entry_point:    usize,
        argument:       usize,
        max_stack_size: usize,
        priority:       u8,
) -> Result<usize, ThreadCreationError> {
    // Make a new thread and push it onto the list.
    let mut thread = Thread::new(process, entry_point, argument, max_stack_size, priority)?;
    let id = thread.id();
    loop {
        match MOVING_THREADS.insert_head(thread) {
            Ok(()) => break,
            Err(x) => thread = x // We moved this into `insert_head`, so we need to move it back.
        };
    }
    Ok(id)
}

// The total number of threads that currently exist.
static TOTAL_THREADS: AtomicUsize = AtomicUsize::new(0);

// The sum of the priorities of all currently running threads. (Sleeping and blocking threads
// aren't included in this sum.)
static TOTAL_PRIORITY: AtomicU32 = AtomicU32::new(0);

// TODO: Blocking threads shouldn't be in any ThreadQueues or circular arrays. Instead,
// they should be handed off to whatever system will wake them up.

// A pointer to the first thread that is waiting to be moved to a CPU. These are stored in a linked
// list with LIFO semantics, so all insertions and removals are done at the head. The list is
// almost always empty, so there shouldn't be any starvation.
static MOVING_THREADS: Semaphore<AtomicLinkedList<Thread<File>>> = AtomicLinkedList::new();

// A place to temporarily store threads that are sleeping for a particular amount of time. These
// are sorted by wake-up time, so that any that should wake up now are at the head of the list.
// TODO: static SLEEPING_THREADS: Semaphore<AtomicLinkedList<Thread<File>>> = AtomicLinkedList::new();
