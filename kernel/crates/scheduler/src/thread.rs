/* Copyright (c) 2022-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

use {
    alloc::{
        boxed::Box,
        vec::Vec,
        sync::Arc,
    },
    core::{
        fmt,
        mem,
        ptr,
        sync::atomic::Ordering,
    },
    io::{Read, Seek},
    super::{TOTAL_THREADS, TOTAL_PRIORITY, Process},
};
#[cfg(target_arch = "aarch64")]
use {
    alloc::alloc::AllocError,
    core::{
        convert::TryFrom,
        ffi::c_void,
        num::NonZeroUsize,
    },
    fs::File,
    memory::virt::paging,
    time::SystemTime,
};

#[cfg(target_arch = "aarch64")]
// enter_userspace(page_table, spsr, entry_point, trampoline_stack_ptr, thread) -> ThreadStatus
type EnterUserspaceFn<T> = extern "C" fn(*const c_void, u64, usize, usize, *const Thread<T>) -> u8;

extern "C" {
    /// Saves the kernel's state, loads the state of the thread from the given parameters, and
    /// jumps into userspace, then returns the thread's updated running state.
    #[cfg(target_arch = "aarch64")]
    fn enter_userspace(
        page_table: *const c_void,
        spsr: u64,
        entry_point: usize,
        trampoline_stack_ptr: usize,
        thread: *const c_void,
    ) -> u8;
}

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
        Terminated = 255,
    }
}

#[cfg(target_arch = "aarch64")]
/// Represents the current state of a thread of execution.
#[derive(Debug)]
pub struct Thread<T: Read+Seek> {
    /// The process to which this thread belongs.
    pub process: Arc<Process<T>>,

    /// The "raw" timestamp (i.e. as if `SystemTime::set_now` had never been called) at which the
    /// thread should wake up, if it's currently sleeping.
    pub wake_time: SystemTime,

    priority: u8,              // The thread's priority. A higher value means longer time slices.

    spsr: u64,                 // Saved Program Status Register (a snapshot of PSTATE)
    elr: usize,                // Exception Link Register (where the thread will start or continue running)

    // A place to store the thread's general-purpose registers when we've switched away from it.
    // This is managed almost entirely by ASM functions; we just need to ensure it's the right size
    // and has the right initial contents.
    register_store: [u64; 32],

    /// A place to store a time in case reading it all requires multiple system calls.
    pub saved_time: SystemTime,
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
    data: core::marker::PhantomData<T>,
}

impl<T: Read+Seek> Thread<T> {
    /// Returns this thread's unique Thread ID.
    pub fn id(&self) -> usize {
        (self as *const Self).expose_addr()
    }

    /// Converts a Thread ID into a raw pointer to a thread. Note that the thread is *NOT*
    /// guaranteed to actually exist, hence the raw pointer. Dereferencing it is unsafe unless this
    /// CPU currently owns the thread.
    ///
    /// # Returns
    /// The raw pointer, or `Err` if the Thread ID is invalid.
    pub fn from_id(id: usize) -> Result<*const Thread<T>, ()> {
        if id % mem::align_of::<Thread<T>>() == 0 {
            Ok(ptr::from_exposed_addr(id))
        } else {
            Err(())
        }
    }
}

#[cfg(target_arch = "aarch64")]
impl<T: Read+Seek> Thread<T> {
    /// Spawns a new thread that will run in the given process, starting at the given entry point,
    /// with a stack that is at least the given size.
    pub fn new(
            process:            Arc<Process<T>>,
            entry_point:        usize,
            argument:           usize,
            mut max_stack_size: usize,
            priority:           u8,
    ) -> Result<Box<Thread<T>>, ThreadCreationError> {
        TOTAL_THREADS.fetch_add(1, Ordering::Release);
        TOTAL_PRIORITY.fetch_add(priority.into(), Ordering::Release);

        // Reserve the page table entries for the stack without allocating any physical memory for
        // now.
        let page_size = paging::page_size();
        max_stack_size = max_stack_size.wrapping_add(page_size - 1) / page_size * page_size;
        let stack_empty_ptr = if let Some(max_stack_size) = NonZeroUsize::new(max_stack_size) {
            let stack_base = process.exec_image.page_table().map_zeroed(None, max_stack_size)
                .map_err(|()| ThreadCreationError::StackAddrConflict)?;
            stack_base + max_stack_size.get()
        } else {
            0
        };

        Box::try_new(Thread {
            process,
            wake_time: SystemTime::UNIX_EPOCH,
            priority,
            spsr: 0, // TODO: This might not be 0 for a new Aarch32 thread.
            elr: entry_point,
            register_store: Self::initial_register_store(argument, stack_empty_ptr),
            saved_time: SystemTime::now()
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

    fn initial_register_store(x0: usize, stack_ptr: usize) -> [u64; 32] {
        [u64::try_from(x0).unwrap(), 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, 0,
         0, 0, 0, 0, 0, 0, 0, u64::try_from(stack_ptr).unwrap()]
    }

    pub(crate) fn priority(&self) -> u8 {
        self.priority
    }
}

#[cfg(target_arch = "aarch64")]
impl Thread<File> {
    // TODO: Find a way to make this generic over all `Thread`s, not just those that use `File`s.
    // Transfers control to this thread.
    pub(crate) fn run(&mut self) -> ThreadStatus {
        // Switch to the thread's address space and transfer control to it.
        let status = unsafe {
            let func: EnterUserspaceFn<File> = mem::transmute(paging::trampoline(enter_userspace as *const ()));
            match ThreadStatus::try_from((func)(
                &*self.process.exec_image.page_table().table_ptr(),
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
    /// Spawns a new thread that will run in the given process, starting at the given entry point,
    /// with a stack that is at least the given size.
    pub fn new(
            _process:        Arc<Process<T>>,
            _entry_point:    usize,
            _argument:       usize,
            _max_stack_size: usize,
            _priority:       u8,
    ) -> Result<Box<Thread<T>>, ThreadCreationError> {
        // TODO
        unimplemented!();
    }

    pub(crate) fn run(&mut self) -> ThreadStatus {
        // TODO
        unimplemented!();
    }

    pub(crate) fn priority(&self) -> u8 {
        self.priority
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
