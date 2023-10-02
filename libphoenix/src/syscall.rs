/* Copyright (c) 2021-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#[cfg(not(feature = "kernelspace"))]
use {
    alloc::vec::Vec,
    core::{
        arch::asm,
        mem::{self, MaybeUninit},
    },
    crate::{
        ipc::FileDescriptor,
        process::ProcessId,
        serde::{DefaultSerializer, Serializer, SerializeError, serialize_object},
    },
};
use core::convert::TryFrom;

#[cfg(not(feature = "kernelspace"))]
const DEFAULT_STACK_SIZE: usize = 0x0001_0000;
#[cfg(not(feature = "kernelspace"))]
const DEFAULT_PRIORITY: u8 = 10;

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
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
pub extern "C" fn thread_sleep(milliseconds: u64) {
    unsafe {
        asm!(
            "svc 0x0001",
            in("x2") milliseconds,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
#[export_name = "thread_spawn"]
extern "C" fn thread_spawn_ffi(
    entry_point: extern "C" fn(usize) -> !,
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

#[cfg(not(feature = "kernelspace"))]
extern "C" fn call_rust_abi(f: usize) -> ! {
    let f = unsafe { mem::transmute::<_, fn()>(f) };
    f();
    thread_exit(0);
}

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
/// Spawns a new process from the executable file at the given path.
///
/// This function works at about the same level as [`posix_spawn`], so it isn't quite as versatile
/// as `fork` and `exec`. But for the usual case of running another program, this should be quite
/// sufficient.
///
/// # Returns
/// The ID of the newly spawned process, or `Err` if it couldn't be spawned.
///
/// [`posix_spawn`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/posix_spawn.html
pub fn process_spawn(path: &str, argv: &[&[u8]], file_descriptors: &[FileDescriptor]) -> Result<ProcessId, ProcessSpawnError> {
    // Serialize the data.
    let mut data = Vec::new();
    data.extend_from_slice(&DEFAULT_STACK_SIZE.to_ne_bytes()[..]);
    data.push(DEFAULT_PRIORITY);
    data.extend_from_slice(&[0; 7][..]);
    data.extend_from_slice(&argv.len().to_ne_bytes()[..]);
    data.extend_from_slice(&(argv as *const [&[u8]]).expose_addr().to_ne_bytes()[..]);

    let argv_strings_size: usize = argv.iter().map(|s| s.len() + 1).sum();
    data.extend_from_slice(&argv_strings_size.to_ne_bytes()[..]);

    for i in 0 .. argv.len() {
        data.extend_from_slice(&argv[i].len().to_ne_bytes()[..]);
    }

    let mut serializer = DefaultSerializer::new();
    serialize_object!(&mut serializer, {
        "fds" => |serializer| serializer.list(file_descriptors)
    }).map_err(|e| ProcessSpawnError::SerializeError(e))?;
    data.append(&mut serializer.finish());

    // Spawn the process.
    let (pid, errno);
    unsafe {
        asm!(
            "svc 0x0101",
            in("x2") path.as_bytes() as *const [u8] as *const u8 as usize,
            in("x3") path.len(),
            in("x4") &data[..] as *const [u8] as *const u8 as usize,
            in("x5") data.len(),
            lateout("x0") pid,
            lateout("x1") errno,
            options(nomem, nostack, preserves_flags),
        );
    }

    ProcessId::new(pid).ok_or(ProcessSpawnError::Errno(errno))
}

#[cfg(not(feature = "kernelspace"))]
/// An error that can occur as a result of trying and failing to spawn a process.
#[derive(Debug)]
pub enum ProcessSpawnError {
    /// An error occurred when serializing the data for the process.
    SerializeError(SerializeError),

    /// An error occurred when trying to start the process.
    ///
    /// Compare the contained value to variants of [`Errno`](crate::posix::errno::Errno) for more
    /// information about the error.
    Errno(u64),
}

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
/// Frees a block of memory that was allocated by a system call.
///
/// # Aborts the thread
/// If the given address is not the address of the first byte of an allocated block. (The most
/// likely reason for this to happen is that the memory has already been freed.)
#[no_mangle]
pub unsafe extern "C" fn memory_free(ptr: *mut MaybeUninit<u8>) {
    asm!(
        "svc 0x0300",
        in("x2") ptr,
        options(nomem, nostack, preserves_flags),
    );
}

#[cfg(not(feature = "kernelspace"))]
/// Allocates a new block of virtual memory with the given size and alignment.
///
/// This is only meant for use by the global allocator. Other userspace code should allocate
/// through that allocator.
///
/// # Returns
/// A pointer to the allocated block, or null if the allocation failed.
#[no_mangle]
pub extern "C" fn memory_alloc(size: usize, align: usize) -> *mut MaybeUninit<u8> {
    let addr: *mut MaybeUninit<u8>;
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

#[cfg(not(feature = "kernelspace"))]
/// Allocates a new block of memory with the given size and alignment. The memory is guaranteed
/// to remain resident at the same address in physical memory until it is freed, and it is
/// guaranteed that the highest physical address in the block fits within a `max_bits`-bit
/// unsigned integer.
///
/// The purpose of this system call is to allow a driver to allocate a buffer and hand that
/// buffer to its device.
///
/// # Returns
/// A struct containing the virtual pointer to and physical address of the allocated block. They
/// are null and 0 if and only if allocation fails.
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
    let virt: *mut MaybeUninit<u8>;
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

#[cfg(not(feature = "kernelspace"))]
/// Allocates a new block of shared virtual memory with the given size.
///
/// This is a low-level primitive for inter-process communication and should probably not be used
/// directly. Instead, use one of the abstractions in the [`ipc` module].
///
/// The memory will not be shared with any existing processes, but any child process created after
/// the memory is allocated can call [`memory_access_shared`] to get read-write access to it.
///
/// Freeing the memory is done via [`memory_free`]. The memory will not actually be freed until
/// every process that has gained access has also called `memory_free`.
///
/// # Returns
/// A pointer to the allocated block, or null if the allocation failed.
///
/// [`ipc` module]: super::ipc
#[no_mangle]
pub extern "C" fn memory_alloc_shared(size: usize) -> *mut MaybeUninit<u8> {
    let addr: *mut MaybeUninit<u8>;
    unsafe {
        asm!(
            "svc 0x0303",
            in("x2") size,
            lateout("x0") addr,
            options(nomem, nostack, preserves_flags),
        );
    }
    addr
}

#[cfg(not(feature = "kernelspace"))]
/// Requests access to a block of shared virtual memory.
///
/// This is a low-level primitive for inter-process communication and should probably not be used
/// directly. Instead, use one of the abstractions in the [`ipc` module].
///
/// `orig_addr` and `size` must be the address and size of a block of shared memory as returned by
/// [`memory_alloc_shared`]. Note that `orig_addr` is the address of the block in the _original_
/// process's address space, hence the name. The pointer returned by this system call is not
/// guaranteed to have the same address.
///
/// Any process that gains access to shared memory is responsible for eventually calling
/// [`memory_free`]. The memory will not actually be freed until every process that has gained
/// access has also called `memory_free`.
///
/// # Returns
/// A pointer to the block of shared memory, or null if the block can't be accessed (e.g. if it has
/// already been freed).
///
/// [`ipc` module]: super::ipc
#[no_mangle]
pub extern "C" fn memory_access_shared(orig_addr: usize, size: usize) -> *mut MaybeUninit<u8> {
    let addr: *mut MaybeUninit<u8>;
    unsafe {
        asm!(
            "svc 0x0304",
            in("x2") orig_addr,
            in("x3") size,
            lateout("x0") addr,
            options(nomem, nostack, preserves_flags),
        );
    }
    addr
}

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
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

#[cfg(not(feature = "kernelspace"))]
/// Used for packaging a virtual address and a physical address into a single return value.
#[derive(Debug)]
#[repr(C)]
pub struct VirtPhysAddr {
    /// A pointer to the value using a virtual address.
    pub virt: *mut MaybeUninit<u8>,
    /// The physical address.
    pub phys: usize,
}

#[cfg(not(feature = "kernelspace"))]
impl VirtPhysAddr {
    /// Returns `true` if the address is null.
    pub fn is_null(&self) -> bool {
        assert_eq!(self.virt.is_null(), self.phys == 0);
        self.virt.is_null()
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
