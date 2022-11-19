/* Copyright (c) 2021-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

/// This module defines how the Phoenix kernel responds to system calls.

use {
    core::{
        alloc::AllocError,
        convert::{TryFrom, TryInto},
        ffi::c_void,
        mem,
        num::NonZeroUsize,
        time::Duration,
    },
    volatile::{
        Volatile,
        access::WriteOnly,
    },
    libphoenix::{
        profiler, profiler_probe, profiler_setup,
        syscall::TimeSelector,
    },
    devices::DEVICES,
    fs::File,
    io::{println, Read},
    memory::{
        allocator::AllMemAlloc,
        phys::ptr::PhysPtr,
        virt::paging,
    },
    scheduler::{Thread, ThreadStatus},
    shared::ffi_enum,
    time::SystemTime,
    userspace::UserspaceStr,
    super::exceptions::Response,
};

extern {
    static __profile_start: c_void;
    static __profile_end:   c_void;
}

profiler_setup!();

pub(crate) fn handle_system_call(
        thread: Option<&mut Thread<File>>,
        syscall: u16,
        args: &[usize; 4],
        mut result: Volatile<&mut [usize; 2], WriteOnly>,
) -> Response {
    let result1 = result.map_mut(|x| &mut x[0]);
    match SystemCall::try_from(syscall) {
        Ok(SystemCall::Thread_Exit)  => thread_exit(thread, args[0]),
        Ok(SystemCall::Thread_Sleep) => thread_sleep(thread, args[0]),
        Ok(SystemCall::Thread_Spawn) => thread_spawn(thread, args[0], args[1] as u8, args[2], result1),

        Ok(SystemCall::Process_Exit) => process_exit(thread, args[0]),

        Ok(SystemCall::Device_Claim) => device_claim(thread, args[0], args[1], result1),

        Ok(SystemCall::Memory_Free) => memory_free(thread, args[0]),
        Ok(SystemCall::Memory_Alloc) => memory_alloc(thread, args[0], args[1], result1),
        Ok(SystemCall::Memory_AllocPhys) => memory_alloc_phys(thread, args[0], args[1], args[2], result),
        Ok(SystemCall::Memory_PageSize) => memory_page_size(result1),

        Ok(SystemCall::Time_NowUnix) => time_now_unix(thread, args[0], args[1], result1),
        Ok(SystemCall::Time_NowUnixNanos) => time_now_unix_nanos(thread, args[0], args[1], result1),
        Ok(SystemCall::Time_ViewKernelProfile) => time_view_kernel_profile(thread, result1),
        Ok(SystemCall::Time_ResetKernelProfile) => time_reset_kernel_profile(thread, result1),

        // TODO: Remove all of these temporary system calls.
        Ok(SystemCall::Temp_PutChar) => temp_putchar(args[0]),
        Ok(SystemCall::Temp_GetChar) => temp_getchar(result1),

        Err(e) => {
            // TODO: Maybe distinguish between normal termination and a crash.
            // TODO: Send a signal to the thread's parent or something, instead of printing.
            println!("{}", e);
            process_exit(thread, usize::MAX) // TODO: Use a named constant for the failure code.
        },
    }
}

ffi_enum! {
    #[repr(u16)]
    #[allow(non_camel_case_types)]
    enum SystemCall {
        Thread_Exit             = 0x0000,
        Thread_Sleep            = 0x0001,
        Thread_Spawn            = 0x0002,

        Process_Exit            = 0x0100,

        Device_Claim            = 0x0200,

        Memory_Free             = 0x0300,
        Memory_Alloc            = 0x0301,
        Memory_AllocPhys        = 0x0302,
        Memory_PageSize         = 0x0380,

        Time_NowUnix            = 0x0400,
        Time_NowUnixNanos       = 0x0401,
        Time_ViewKernelProfile  = 0x0480,
        Time_ResetKernelProfile = 0x0481,

        Temp_PutChar            = 0xff00,
        Temp_GetChar            = 0xff01,
    }
}

// Terminates the current thread, returning to the kernel's state from before the thread started
// running.
fn thread_exit(thread: Option<&mut Thread<File>>, status: usize) -> Response {
    assert!(!thread.is_none(), "attempted to terminate a kernel thread");
    if status != 0 {
        // FIXME: Handle thread return values.
        unimplemented!("thread return value");
    }
    Response::leave_userspace(ThreadStatus::Terminated)
}

// Puts the current thread to sleep for at least the specified amount of time. Asking to sleep for
// 0 seconds results in forfeiting the rest of this time slice.
fn thread_sleep(thread: Option<&mut Thread<File>>, milliseconds: usize) -> Response {
    let thread = thread.expect("attempted to put a kernel thread to sleep");
    let status;
    if milliseconds > 0 {
        thread.wake_time = SystemTime::now_raw() + Duration::from_millis(milliseconds.try_into().unwrap());
        status = ThreadStatus::Sleeping;
    } else {
        status = ThreadStatus::Running;
    }
    Response::leave_userspace(status)
}

// Spawns a new thread in the same process as the calling thread, using the given address as an
// entry point. The entry point should be the beginning of a function that takes no arguments and
// never returns. (Instead, it should use a system call to terminate itself.)
fn thread_spawn(
        thread: Option<&mut Thread<File>>,
        entry_point: usize,
        mut priority: u8,
        max_stack_size: usize,
        mut handle: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let parent_thread = thread.expect("attempted to spawn a new kernel thread");
    let entry_point = usize::try_from(entry_point).unwrap();
    // TODO: A priority of 0 should maybe mean real-time (i.e. cooperative scheduling only). We'll
    // need to adjust the load-balancing logic to account for that.
    if priority == 0 {
        priority = 1;
    }
    handle.write(
        scheduler::spawn_thread(parent_thread.exec_image.clone(), entry_point, max_stack_size, priority).unwrap_or(0)
    );
    profiler_probe!(ENTRANCE);
    Response::eret()
}


// Terminates the process containing the current thread, thereby terminating every thread in that
// process.
fn process_exit(thread: Option<&mut Thread<File>>, status: usize) -> Response {
    assert!(!thread.is_none(), "attempted to terminate a kernel thread's process");
    if status != 0 {
        // FIXME: Handle process return values.
        unimplemented!("process return value");
    }
    // FIXME: Exit the process, not just the current thread.
    Response::leave_userspace(ThreadStatus::Terminated)
}


// Looks up a device by its path in the device tree, gives the process access to its registers, and
// returns a pointer to an object describing the device.
//
// Requires the permission `own device <device_name>`.
//
// If the device doesn't exist, or the process lacks the necessary permission (and the user doesn't
// grant it that permission), this returns a null pointer.
fn device_claim(
        thread: Option<&mut Thread<File>>,
        dev_name_userspace_addr: usize,
        dev_name_len: usize,
        mut userspace_addr: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let thread = thread.expect("kernel thread attempted to get a device");

    let root_page_table = thread.exec_image.page_table();

    let dev_path = match UserspaceStr::from_raw_parts(
            root_page_table,
            thread.exec_image.virt_reader(),
            dev_name_userspace_addr,
            dev_name_len,
    ) {
        Some(path) => path,
        None => return Response::leave_userspace(ThreadStatus::Terminated) // Part of the argument is unmapped.
    };
    userspace_addr.write(DEVICES.claim_device(dev_path, thread.exec_image.page_table()).unwrap_or(0));

    profiler_probe!(ENTRANCE);
    Response::eret()
}


// Frees a block of memory starting at the given address that was allocated by a system call. Kills
// the thread if the address doesn't refer to such a block.
fn memory_free(
    thread: Option<&mut Thread<File>>,
    _userspace_addr: usize,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let _thread = thread.expect("kernel thread attempted to free memory with a system call");

    // TODO: Locate the block with the given address that is owned by this thread's process. If this can't
    //       be done in constant time, do it asynchronously.
    // TODO: Drop that block.

    profiler_probe!(ENTRANCE);
    Response::eret()
}

// Asynchronously allocates a block of memory containing at least `size` bytes with at least the
// given alignment. Returns the userspace address of the block, or null to indicate failure.
fn memory_alloc(
    thread: Option<&mut Thread<File>>,
    size: usize,
    align: usize,
    mut userspace_addr: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let thread = thread.expect("kernel thread attempted to allocate memory with a system call");
    let page_size = paging::page_size();

    // FIXME: Do this asynchronously. Memory allocation has unbounded time complexity, and we can't
    //        pre-empt the thread during a system call.
    let mut maybe_block = match AllMemAlloc.malloc::<u8>(
            size.saturating_add(page_size - 1) / page_size * page_size,
            NonZeroUsize::new(usize::max(align, page_size)).unwrap()
    ) {
        Ok(block) => Some(block),
        Err(AllocError) => None,
    };

    let root_page_table = thread.exec_image.page_table();

    userspace_addr.write(match maybe_block {
        Some(ref block) => {
            match root_page_table.map(
                    block.base().as_addr_phys(),
                    None,
                    NonZeroUsize::new(block.size()).unwrap(),
                    memory::phys::RegionType::Ram
            ) {
                Ok(addr) => addr,
                Err(()) => {
                    maybe_block = None;
                    0
                }
            }
        },
        None => 0,
    });

    // FIXME: Instead of forgetting the block, attach it to the process.
    mem::forget(maybe_block);

    profiler_probe!(ENTRANCE);
    Response::eret()
}

// Asynchronously allocates a physically contiguous block of memory containing at least `size` bytes
// with at least the given alignment. Returns both the physical and the virtual address of the
// block. This memory is guaranteed to stay resident until it is freed. On failure, both addresses
// are null.
//
// The physical address of every byte in the allocated block is guaranteed not to overflow an
// unsigned binary number of length `max_bits`.
fn memory_alloc_phys(
    thread: Option<&mut Thread<File>>,
    size: usize,
    align: usize,
    max_bits: usize,
    mut userspace_and_phys_addrs: Volatile<&mut [usize; 2], WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let thread = thread.expect("kernel thread attempted to allocate memory with a system call");
    let page_size = paging::page_size();

    // FIXME: Do this asynchronously. Memory allocation has unbounded time complexity, and we can't
    //        pre-empt the thread during a system call.
    let mut maybe_block = match AllMemAlloc.malloc_low::<u8>(
            size.saturating_add(page_size - 1) / page_size * page_size,
            NonZeroUsize::new(usize::max(align, page_size)).unwrap(),
            max_bits
    ) {
        Ok(block) => Some(block),
        Err(AllocError) => None,
    };

    let root_page_table = thread.exec_image.page_table();

    userspace_and_phys_addrs.write(match maybe_block {
        Some(ref block) => {
            let phys_addr = block.base().as_addr_phys();
            match root_page_table.map(
                    phys_addr,
                    None,
                    NonZeroUsize::new(block.size()).unwrap(),
                    memory::phys::RegionType::Ram
            ) {
                Ok(virt_addr) => [virt_addr, phys_addr],
                Err(()) => {
                    maybe_block = None;
                    [0, 0]
                }
            }
        },
        None => [0, 0]
    });

    // FIXME: Instead of forgetting the block, attach it to the process.
    mem::forget(maybe_block);

    profiler_probe!(ENTRANCE);
    Response::eret()
}

// Returns the size of a page.
fn memory_page_size(
    mut result: Volatile<&mut usize, WriteOnly>,
) -> Response {
    result.write(paging::page_size());
    Response::eret()
}

// Returns the time as a UNIX timestamp. `time_selector` and `shift_amount` exist to allow for
// different word sizes without inaccuracies or huge performance penalties. The userspace program
// can call this with `TimeSelector::Now` and `shift_amount = 0` at first, followed by multiple
// calls with `TimeSelector::Saved` and, e.g., `shift_amount = 32` to get the higher bytes.
fn time_now_unix(
    thread: Option<&mut Thread<File>>,
    time_selector: usize,
    shift_amount: usize,
    mut result: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let thread = thread.expect("kernel thread attempted to read the time with a system call");

    let time_selector = TimeSelector::try_from(time_selector).unwrap_or(TimeSelector::Now);
    match time_selector {
        TimeSelector::Now => thread.saved_time = time::SystemTime::now(),
        TimeSelector::Saved => {}
    }

    let time_since_epoch = thread.saved_time.duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    result.write((time_since_epoch.as_secs() >> (shift_amount as u64)) as usize);

    profiler_probe!(ENTRANCE);
    Response::eret()
}

// Returns the time as the number of nanoseconds since the UNIX epoch. `time_selector` and
// `shift_amount` exist to allow for different word sizes without inaccuracies or huge performance
// penalties. The userspace program can call this with `TimeSelector::Now` and `shift_amount = 0` at
// first, followed by multiple calls with `TimeSelector::Saved` and, e.g., `shift_amount = 32` to
// get the higher bytes.
fn time_now_unix_nanos(
    thread: Option<&mut Thread<File>>,
    time_selector: usize,
    shift_amount: usize,
    mut result: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let thread = thread.expect("kernel thread attempted to read the time with a system call");

    let time_selector = TimeSelector::try_from(time_selector).unwrap_or(TimeSelector::Now);
    match time_selector {
        TimeSelector::Now => thread.saved_time = time::SystemTime::now(),
        TimeSelector::Saved => {}
    }

    let time_since_epoch = thread.saved_time.duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    result.write((time_since_epoch.as_nanos() >> (shift_amount as u64)) as usize);

    profiler_probe!(ENTRANCE);
    Response::eret()
}

// Maps the kernel's internal performance profile to a set of contiguous userspace pages and returns
// the base address of the first page.
fn time_view_kernel_profile(
    thread: Option<&mut Thread<File>>,
    mut userspace_addr: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let thread = thread.expect("kernel thread attempted to read the kernel's time profile with a system call");

    let phys_base = PhysPtr::from(unsafe { &__profile_start as *const _ }).as_addr_phys();
    let size = unsafe { &__profile_end as *const _ as usize - &__profile_start as *const _ as usize };
    let page_size = paging::page_size();

    assert_eq!(phys_base % page_size, 0, "misaligned kernel profile (address = {phys_base:#018x})");
    assert_eq!(size % page_size, 0, "wrongly sized kernel profile (size = {size:#018x})");

    let root_page_table = thread.exec_image.page_table();

    userspace_addr.write(if let Some(size) = NonZeroUsize::new(size) {
        // FIXME: Remember where this is mapped so the process can request that it be unmapped.
        root_page_table.map(phys_base, None, size, memory::phys::RegionType::Rom).unwrap_or(0)
    } else {
        0
    });

    profiler_probe!(ENTRANCE);
    Response::eret()
}

// Resets the kernel's performance profile.
fn time_reset_kernel_profile(
    thread: Option<&mut Thread<File>>,
    mut result: Volatile<&mut usize, WriteOnly>,
) -> Response {
    profiler_probe!(=> ENTRANCE);
    let _thread = thread.expect("kernel thread attempted to reset the kernel's time profile with a system call");

    // FIXME: Add some security around this. We don't want just any old program resetting the profile
    // and messing with any measurements that might be happening right now.

    profiler::reset();

    result.write(0); // Placeholder for maybe an actual return value

    profiler_probe!(ENTRANCE);
    Response::eret()
}


// TODO: Remove these temporary system calls.
fn temp_putchar(c: usize) -> Response {
    io::print!("{}", char::try_from(c as u32).unwrap_or('?'));
    Response::eret()
}
fn temp_getchar(mut c: Volatile<&mut usize, WriteOnly>) -> Response {
    let mut stdin = io::STDIN.try_lock().unwrap();
    let mut buffer = [0u8; 1];
    stdin.read_exact(&mut buffer[ .. ])
        .expect("error reading from standard input");
    io::print!("{}", core::str::from_utf8(&buffer).unwrap_or("?"));
    c.write(buffer[0].into());
    Response::eret()
}
