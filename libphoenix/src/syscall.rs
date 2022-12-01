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

//! This module defines a function to encapsulate each system call the Phoenix kernel understands.

use {
    core::{
        arch::asm,
        convert::TryFrom,
        mem,
    },
};

/// Ends the currently running thread with the given status.
///
/// This function is exactly equivalent to [`process_exit`] if the process has only one running
/// thread.
///
/// # Returns
/// Never returns.
///
/// # Example
/// ```no_run
/// fn main() {
///     let child_thread = thread_spawn(do_work, priority, stack_size);
///     let status = child_thread.join();
///     assert_eq!(status, 42);
/// }
///
/// fn do_work() {
///     thread_exit(42);
///     # #[allow(dead_code)]
///     panic!("This line will never be reached.");
/// }
/// ```
#[no_mangle]
pub extern "C" fn thread_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "svc 0x0000",
            in("x2") status as usize,
            options(nomem, nostack, preserves_flags, noreturn),
        );
    }
}

/// Halts the currently running thread for the given duration.
///
/// The given duration is a lower bound. The thread may (and likely will) be inactive for slightly
/// longer than the given duration, but it will not wake up early.
///
/// If `milliseconds` is 0, the thread still halts, but for the shortest amount of time possible.
/// This is useful for avoiding busy-waiting, as it allows other threads to run.
///
/// # Example
/// ```no_run
/// let first_timestamp = time_now();
/// thread_sleep(1000);
/// assert!(time_now() >= first_timestamp + 1000);
/// ```
#[no_mangle]
pub extern "C" fn thread_sleep(milliseconds: usize) {
    unsafe {
        asm!(
            "svc 0x0001",
            in("x2") milliseconds,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Spawns a new thread with the given entry point, priority, and stack size.
///
/// This function is not marked as public because the [`Thread`] type provides a more idiomatic
/// way to call it.
///
/// # Returns
/// The thread's handle.
pub(crate) fn thread_spawn(entry_point: fn(), priority: u8, stack_size: usize) -> usize {
    thread_spawn_ffi(call_rust_abi, entry_point as usize, priority, stack_size)
}

#[export_name = "thread_spawn"]
extern "C" fn thread_spawn_ffi(
    entry_point: extern "C" fn(usize),
    argument: usize,
    priority: u8,
    stack_size: usize,
) -> usize {
    let handle: usize;
    unsafe {
        asm!(
            "svc 0x0002",
            in("x2") entry_point as usize,
            in("x3") argument,
            in("x4") priority,
            in("x5") stack_size,
            lateout("x0") handle,
            options(nomem, nostack, preserves_flags),
        );
    }
    handle
}

extern "C" fn call_rust_abi(f: usize) {
    let f = unsafe { mem::transmute::<_, fn()>(f) };
    f()
}

/// Ends the currently running process with the given status.
///
/// Every thread in the process is immediately terminated.
///
/// # Returns
/// Never returns.
///
/// # Example
/// ```no_run
/// println!("Successful exit from the process");
/// process_exit(0);
/// # #[allow(dead_code)]
/// panic!("This code will never be reached.");
/// ```
#[no_mangle]
pub extern "C" fn process_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "svc 0x0100",
            in("x2") status as usize,
            options(nomem, nostack, preserves_flags, noreturn),
        );
    }
}

/// Looks up a physical device by name and claims ownership of it if it exists.
///
/// This function is intended for use only by `libdriver`, which builds further abstractions on
/// top of it.
///
/// # Returns
/// The address of the object describing the device, or `0` if (a) the device doesn't exist
/// or (b) this process doesn't have permission to claim it.
///
/// # Required permissions
/// * `own device <name>`
pub fn device_claim(name: &str) -> usize {
    device_claim_ffi(name as *const str as *const u8, name.len())
}

#[export_name = "device_claim"]
extern "C" fn device_claim_ffi(name: *const u8, len: usize) -> usize {
    let device_addr: usize;
    unsafe {
        asm!(
            "svc 0x0200",
            in("x2") name as usize,
            in("x3") len,
            lateout("x0") device_addr,
            options(nomem, nostack, preserves_flags),
        );
    }
    device_addr
}

/// Frees a block of memory that was allocated by a system call.
///
/// # Aborts the thread
/// If the given address is not the address of the first byte of an allocated block. (The most
/// likely reason for this to happen is that the memory has already been freed.)
#[no_mangle]
pub extern "C" fn memory_free(addr: usize) {
    unsafe {
        asm!(
            "svc 0x0300",
            in("x2") addr,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Allocates a new block of virtual memory with the given size and alignment.
///
/// This is only meant for use by the global allocator. Other userspace code should allocate
/// through that allocator.
///
/// # Returns
/// The address of the allocated block, or `0` if the allocation failed.
#[no_mangle]
pub extern "C" fn memory_alloc(size: usize, align: usize) -> usize {
    let addr: usize;
    unsafe {
        asm!(
            "svc 0x0301",
            in("x2") size,
            in("x3") align,
            lateout("x0") addr,
            options(nomem, nostack, preserves_flags),
        );
    }
    addr
}

/// Allocates a new block of memory with the given size and alignment. The memory is guaranteed
/// to remain resident at the same address in physical memory until it is freed, and it is
/// guaranteed that the highest physical address in the block fits within a `max_bits`-bit
/// unsigned integer.
///
/// The purpose of this system call is to allow a driver to allocate a buffer and hand that
/// buffer to its device.
///
/// # Returns
/// A struct containing the virtual and physical addresses of the allocated block. If allocation
/// fails, both addresses are `0`. Otherwise, neither is `0`.
///
/// # Example
/// ```
/// # async {
/// let addr = memory_alloc_phys(0x1000, 0x1000, 20).await;
/// assert!(addr.phys < 1 << 20);
/// if addr.is_null() {
///     println!("Allocation failed");
/// } else {
///     println!("Allocated 0x1000 bytes at virtual address {:#x}, physical address {:#x}", addr.virt, addr.phys);
/// }
/// # }
/// ```
#[no_mangle]
pub extern "C" fn memory_alloc_phys(size: usize, align: usize, max_bits: usize) -> VirtPhysAddr {
    let virt: usize;
    let phys: usize;
    unsafe {
        asm!(
            "svc 0x0302",
            in("x2") size,
            in("x3") align,
            in("x4") max_bits,
            lateout("x0") virt,
            lateout("x1") phys,
            options(nomem, nostack, preserves_flags),
        );
    }
    VirtPhysAddr { virt, phys }
}

/// Returns the number of bytes in a page.
///
/// A page is the smallest unit of memory that the kernel will allocate for a userspace program, and
/// page size and alignment also sometimes matter for drivers. Most programs don't need to worry
/// about it at all.
///
/// # Example
/// ```norun
/// println!("The size of a page is {} bytes.", memory_page_size());
/// ```
#[no_mangle]
pub extern "C" fn memory_page_size() -> usize {
    let page_size: usize;
    unsafe {
        asm!(
            "svc 0x0380",
            lateout("x0") page_size,
            options(nomem, nostack, preserves_flags),
        );
    }
    page_size
}

/// Returns the current time as a UNIX timestamp (i.e. whole seconds since 1970-01-01T00:00:00).
///
/// # Example
/// ```norun
/// let time1 = time_now_unix();
/// let time2 = time_now_unix();
/// if time2 < time1 {
///     println!("Either the user changed the clock or we just went back in time!");
/// }
/// ```
#[no_mangle]
pub extern "C" fn time_now_unix() -> u64 {
    // FIXME: Make this work with word sizes other than 64 bits.
    const { assert!(mem::size_of::<usize>() == mem::size_of::<u64>()) };

    let unix_time: u64;
    unsafe {
        asm!(
            "svc 0x0400",
            in("x2") TimeSelector::Now as usize,
            in("x3") 0,
            lateout("x0") unix_time,
            options(nomem, nostack, preserves_flags),
        );
    }
    unix_time
}

/// Returns the current time in relation to the UNIX epoch, but measured in nanoseconds
/// (i.e. nanoseconds since 1970-01-01T00:00:00.00).
///
/// # Example
/// ```norun
/// let time1 = time_now_unix_nanos();
/// let time2 = time_now_unix_nanos();
/// if time2 < time1 {
///     println!("Either the user changed the clock or we just went back in time!");
/// }
/// ```
#[no_mangle]
pub extern "C" fn time_now_unix_nanos() -> u64 {
    // FIXME: Make this work with word sizes other than 64 bits.
    const { assert!(mem::size_of::<usize>() == mem::size_of::<u64>()) };

    let unix_time: u64;
    unsafe {
        asm!(
            "svc 0x0401",
            in("x2") TimeSelector::Now as usize,
            in("x3") 0,
            lateout("x0") unix_time,
            options(nomem, nostack, preserves_flags),
        );
    }
    unix_time
}

/// Resets the kernel's internal performance profile. This should be done right before the code
/// that will be measured.
pub fn time_reset_kernel_profile() {
    unsafe {
        asm!(
            "svc 0x0481",
            options(nomem, nostack, preserves_flags),
        );
    }
}


/// *Note:* This function is not designed to be used directly. Applications should use
/// [`crate::profiler::kernel_probes`] instead.
///
/// Maps the kernel's internal performance profile directly to a set of contiguous pages in this
/// process's address space. This is not a snapshot; it is updated in real time as different
/// parts of the kernel are executed.
///
/// Note that the kernel may have been compiled without profiling support. In that case, any call
/// to this function will just return 0, representing a null pointer.
pub fn time_view_kernel_profile() -> usize {
    let profile_addr: usize;
    unsafe {
        asm!(
            "svc 0x0480",
            lateout("x0") profile_addr,
            options(nomem, nostack, preserves_flags),
        );
    }
    profile_addr
}

/// Used for packaging a virtual address and a physical address into a single return value.
#[derive(Debug)]
#[repr(C)]
pub struct VirtPhysAddr {
    /// The virtual address.
    pub virt: usize,
    /// The physical address.
    pub phys: usize,
}

impl VirtPhysAddr {
    /// Returns `true` if the address is null.
    pub fn is_null(&self) -> bool {
        assert_eq!(self.virt == 0, self.phys == 0);
        self.virt == 0
    }
}

/// Used for specifying whether the `time_*` syscalls will use the current time or a time already
/// saved from an earlier call.
#[derive(Debug)]
#[repr(usize)]
pub enum TimeSelector {
    /// Represents the current time.
    Now   = 0,
    /// Represents the last time that the kernel read on behalf of this thread.
    Saved = 1,
}

impl TryFrom<usize> for TimeSelector {
    type Error = ();

    fn try_from(val: usize) -> Result<Self, Self::Error> {
        match val {
            x if x == Self::Now as usize   => Ok(Self::Now),
            x if x == Self::Saved as usize => Ok(Self::Saved),
            _                              => Err(())
        }
    }
}
