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

//! Phoenix's scheduler. It is designed as a simple round-robin scheduler, with each thread
//! executed on a first-come, first-served basis. Threads do have priorities, however. Instead of
//! preempting other threads, high-priority threads simply have longer quanta--that is, they run
//! for longer periods of time.
//!
//! For the sake of efficiency in the scheduler, the exact order of threads is allowed to change,
//! but not in a way that could ever cause a thread's turn to be skipped.

#![no_std]

#![deny(warnings, missing_docs)]

#![cfg_attr(target_arch = "aarch64", feature(allocator_api, asm))]

extern crate alloc;

#[macro_use] extern crate shared;

use {
    alloc::{
        boxed::Box,
        vec::Vec,
        sync::Arc
    },
    core::{
        fmt,
        mem,
        sync::atomic::{AtomicU32, AtomicUsize, Ordering},
        time::Duration
    },
    collections::{AtomicLinkedList, AtomicLinkedListSemaphore},
    exec::ExecImage,
    io::{Read, Seek, printlndebug},
    locks::Semaphore,
    shared::{/*count_cpus, cpu_index,*/ wait_for_event},
    fs::File,
    timers::SCHEDULING_TIMER
};
#[cfg(target_arch = "aarch64")]
use {
    alloc::alloc::AllocError,
    core::{
        convert::TryFrom,
        ffi::c_void,
        num::NonZeroUsize
    },
    memory::virt::paging,
    time::SystemTime
};

#[cfg(target_arch = "aarch64")]
// enter_userspace(page_table, spsr, entry_point, trampoline_stack_ptr, thread) -> ThreadStatus
type EnterUserspaceFn<T> = extern fn(*const c_void, u64, usize, usize, *const Thread<T>) -> u8;

extern {
    /// Saves the kernel's state, loads the state of the thread from the given parameters, and
    /// jumps into userspace, then returns the thread's updated running state.
    #[cfg(target_arch = "aarch64")]
    fn enter_userspace(
        page_table: *const c_void,
        spsr: u64,
        entry_point: usize,
        trampoline_stack_ptr: usize,
        thread: *const c_void
    ) -> u8;
}

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

    let mut priority_sum = thread_queue.iter().fold(0, |s, thread| s + u32::from(thread.priority));

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
            SCHEDULING_TIMER.interrupt_after(quantum(thread_queue[i].priority));
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
                    printlndebug!("Terminating thread {:x}", thread_queue[i].id());
                    priority_sum -= u32::from(thread_queue.swap_remove(i).priority);
                }
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
                        priority_sum += u32::from(thread.priority);
                        thread_queue.push(thread);
                    }
                }
            }
            if cpu_count > 1 && priority_sum >= ideal_priority_sum && thread_queue.len() > 1 {
                // We have more threads than we need.
                let idx = rand(PRNG_PICK_THREAD) as usize % thread_queue.len();
                let thread = &thread_queue[idx];
                if priority_sum - u32::from(thread.priority) >= ideal_priority_sum - rand(PRNG_FUZZY_PRIORITY) as u32 & u32::from(u8::MAX) {
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
        exec_image: Arc<ExecImage<File>>,
        entry_point: usize,
        max_stack_size: usize,
        priority: u8
) -> Result<usize, ThreadCreationError> {
    // Make a new thread and push it onto the list.
    let mut thread = Thread::new(exec_image, entry_point, max_stack_size, priority)?;
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

// A place to temporarily store threads that have been offloaded by an overworked CPU. It doesn't
// have to be big, since another CPU should pick them up soon.
// TODO: static READY_THREADS: AtomicCircularArray<[Option<&'static mut Thread<File>>; 32]> = AtomicCircularArray::
// TODO: Blocking threads shouldn't be in any ThreadQueues or circular arrays. Instead,
// they should be handed off to whatever system will wake them up.

// A pointer to the first thread that is waiting to be moved to a CPU. These are stored in a linked
// list with LIFO semantics, so all insertions and removals are done at the head. The list is
// almost always empty, so there shouldn't be any starvation.
static MOVING_THREADS: Semaphore<AtomicLinkedList<Thread<File>>> = AtomicLinkedList::new();

// A place to temporarily store threads that are sleeping for a particular amount of time. These
// are sorted by wake-up time, so that any that should wake up now are at the head of the list.
// TODO: static SLEEPING_THREADS: Semaphore<AtomicLinkedList<Thread<File>>> = AtomicLinkedList::new();

ffi_enum! {
    #[repr(u8)]
    #[derive(Debug)]
    #[must_use]
    /// Determines whether a given thread is running, sleeping, blocking on I/O, etc. This probably
    /// won't actually be stored with a `Thread` object. Instead, it's returned from `Thread::run` to
    /// tell the scheduler what the thread's new state should be.
    pub enum ThreadStatus {
        /// The thread is currently running or waiting to start running again.
        Running = 0,

        /// The thread is currently sleeping and should not be started until its wake time.
        Sleeping = 1,

        /// The thread has been terminated and will never run again.
        Terminated = 255
    }
}

#[cfg(target_arch = "aarch64")]
/// Represents the current state of a thread of execution.
#[derive(Debug)]
pub struct Thread<T: Read+Seek> {
    /// The "raw" timestamp (i.e. as if `SystemTime::set_now` had never been called) at which the
    /// thread should wake up, if it's currently sleeping.
    pub wake_time: SystemTime,

    /// The image of the executable file that this thread comes from.
    pub exec_image: Arc<ExecImage<T>>,

    priority: u8,              // The thread's priority. A higher value means longer time slices.

    stack_empty_ptr: usize,    // The highest address on the stack (i.e. the value of SP when the stack is empty)
    max_stack_size: usize,     // The stack's logical size, even though we initially allocate less
    spsr: u64,                 // Saved Program Status Register (a snapshot of PSTATE)
    elr: usize,                // Exception Link Register (where the thread will start or continue running)

    // A place to store the thread's general-purpose registers when we've switched away from it.
    // This is managed almost entirely by ASM functions; we just need to ensure it's the right size
    // and has the right initial contents.
    register_store: [u64; 32]
}

#[cfg(target_arch = "aarch64")]
/// Returns a pointer to the given `Thread`'s register store so the registers can be saved or
/// restored.
#[no_mangle]
// TODO: Make this `Thread` reference generic somehow.
extern fn get_thread_register_store(thread: Option<&mut Thread<File>>) -> &mut [u64; 32] {
    let thread = thread.expect("attempted to get the register store for a kernel thread");
    &mut thread.register_store
}

#[cfg(target_arch = "x86_64")]
/// Represents the current state of a thread of execution.
#[derive(Debug)]
pub struct Thread<T: Read+Seek> {
    // TODO
    priority: u8,
    data: core::marker::PhantomData<T>
}

impl<T: Read+Seek> Thread<T> {
    /// Returns this thread's unique Thread ID.
    pub fn id(&self) -> usize {
        self as *const _ as usize
    }

    /// Converts a Thread ID into a raw pointer to a thread. Note that the thread is *NOT*
    /// guaranteed to actually exist, hence the raw pointer. Dereferencing it is unsafe unless this
    /// CPU currently owns the thread.
    ///
    /// # Returns
    /// The raw pointer, or `Err` if the Thread ID is invalid.
    pub fn from_id(id: usize) -> Result<*const Thread<T>, ()> {
        if id % mem::align_of::<Thread<T>>() == 0 {
            Ok(id as *const _)
        } else {
            Err(())
        }
    }
}

#[cfg(target_arch = "aarch64")]
impl<T: Read+Seek> Thread<T> {
    /// Spawns a new thread that will run in the given executable image, starting at the given
    /// entry point, with a stack that is at least the given size.
    pub fn new(
            exec_image:          Arc<ExecImage<T>>,
            entry_point:         usize,
            mut max_stack_size:  usize,
            priority:            u8
    ) -> Result<Box<Thread<T>>, ThreadCreationError> {
        TOTAL_THREADS.fetch_add(1, Ordering::Release);
        TOTAL_PRIORITY.fetch_add(priority.into(), Ordering::Release);

        // Reserve the page table entries for the stack without allocating any physical memory for
        // now.
        let page_size = paging::page_size();
        max_stack_size = max_stack_size.wrapping_add(page_size - 1) / page_size * page_size;
        let stack_empty_ptr = if let Some(max_stack_size) = NonZeroUsize::new(max_stack_size) {
            let stack_base = exec_image.page_table().map_zeroed(None, max_stack_size)
                .map_err(|()| ThreadCreationError::StackAddrConflict)?;
            stack_base + max_stack_size.get()
        } else {
            0
        };

        Box::try_new(Thread {
            wake_time: SystemTime::UNIX_EPOCH,
            exec_image,
            priority,
            stack_empty_ptr,
            max_stack_size,
            spsr: 0, // TODO: This might not be 0 for a new Aarch32 thread.
            elr: entry_point,
            register_store: Self::initial_register_store(stack_empty_ptr)
        })
            .map_err(|AllocError| ThreadCreationError::OutOfMemory)
    }

    // Saves the current state of the thread into this object so that the thread can be resumed
    // later.
    fn save_state(&mut self) {
        unsafe {
            asm!(
                "mrs {}, SPSR_EL1",
                "mrs {}, ELR_EL1",
                out(reg) self.spsr,
                out(reg) self.elr,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    fn initial_register_store(stack_ptr: usize) -> [u64; 32] {
        [0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, u64::try_from(stack_ptr).unwrap()]
    }
}

#[cfg(target_arch = "aarch64")]
impl Thread<File> {
    // TODO: Find a way to make this generic over all `Thread`s, not just those that use `File`s.
    // Transfers control to this thread.
    fn run(&mut self) -> ThreadStatus {
        // Switch to the thread's address space and transfer control to it.
        let status = unsafe {
            let func: EnterUserspaceFn<File> = mem::transmute(paging::trampoline(enter_userspace as *const ()));
            match ThreadStatus::try_from((func)(
                &*self.exec_image.page_table().table_ptr(),
                self.spsr,
                self.elr,
                paging::trampoline_stack_ptr(),
                self
            )) {
                Ok(status) => status,
                Err(e) => panic!("{}", e)
            }
        };
        self.save_state();
        status
    }
}

#[cfg(target_arch = "x86_64")]
impl<T: Read+Seek> Thread<T> {
    /// Spawns a new thread that will run in the given executable image, starting at the given
    /// entry point, with a stack that is at least the given size.
    pub fn new(
            _exec_image:     Arc<ExecImage<T>>,
            _entry_point:    usize,
            _max_stack_size: usize,
            _priority:       u8
    ) -> Result<Box<Thread<T>>, ThreadCreationError> {
        // TODO
        unimplemented!();
    }

    fn run(&mut self) -> ThreadStatus {
        // TODO
        unimplemented!();
    }
}

impl<T: Read+Seek> Drop for Thread<T> {
    fn drop(&mut self) {
        if TOTAL_THREADS.fetch_sub(1, Ordering::AcqRel) == 1 {
            // TODO: There are no more threads left; do something like shutting down the computer.
            panic!("all threads have been terminated");
        }
        TOTAL_PRIORITY.fetch_sub(self.priority.into(), Ordering::AcqRel);
    }
}

/// An array of threads for a CPU to execute.
pub type ThreadQueue<T> = Vec<Box<Thread<T>>>;

/// An error that can occur when creating a new thread.
#[derive(Debug)]
pub enum ThreadCreationError {
    /// An error occurred when trying to map virtual memory for the stack (probably already mapped).
    StackAddrConflict,
    /// The kernel ran out of memory when allocating the thread.
    OutOfMemory
}

impl fmt::Display for ThreadCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: internationalize?
        match *self {
            Self::StackAddrConflict => write!(f, "failed to map virtual memory for the thread's stack"),
            Self::OutOfMemory => write!(f, "ran out of memory when allocating the thread")
        }
    }
}
