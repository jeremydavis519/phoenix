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

//! This module defines a function to encapsulate each system call the Phoenix kernel understands.

use {
    core::mem,
    crate::{
        future::SysCallFuture,
        thread::Thread
    }
};

macro_rules! define_async_syscall {
    (@func $(#[$attr:meta])* pub async fn $name:ident() -> $ret_type:ty => $syscall_num:tt;) => {
        $(#[$attr])*
        pub async fn $name() -> $ret_type {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            // FIXME: Offer some recourse if the kernel returns null (which indicates running out of
            //        memory).
            unsafe {
                const N: usize = mem::size_of::<$ret_type>();
                let retval_vec = SysCallFuture::from_addr::<N>(addr).await;
                let (retval_arrays, remainder) = retval_vec.as_chunks::<N>();
                assert_eq!(retval_arrays.len(), 1, "syscall {}: return value has the wrong size", stringify!($name));
                assert_eq!(remainder.len(), 0, "syscall {}: return value has the wrong size", stringify!($name));
                mem::transmute(retval_arrays[0])
            }
        }
    };

    (@func $(#[$attr:meta])* pub async fn $name:ident(
            $($arg1:ident: $type1:ty)? $(=> {$($conv1:tt)*})?
    ) -> $ret_type:ty => $syscall_num:tt;) => {
        $(#[$attr])*
        pub async fn $name(
                $($arg1: $type1)?
        ) -> $ret_type {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $($arg1)? $($($conv1)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            // FIXME: Offer some recourse if the kernel returns null (which indicates running out of
            //        memory).
            unsafe {
                const N: usize = mem::size_of::<$ret_type>();
                let retval_vec = SysCallFuture::from_addr::<N>(addr).await;
                let (retval_arrays, remainder) = retval_vec.as_chunks::<N>();
                assert_eq!(retval_arrays.len(), 1, "syscall {}: return value has the wrong size", stringify!($name));
                assert_eq!(remainder.len(), 0, "syscall {}: return value has the wrong size", stringify!($name));
                mem::transmute(retval_arrays[0])
            }
        }
    };

    (@func $(#[$attr:meta])* pub async fn $name:ident(
        $($arg1:ident: $type1:ty)? $(=> {$($conv1:tt)*})?,
        $($arg2:ident: $type2:ty)? $(=> {$($conv2:tt)*})?
    ) -> $ret_type:ty => $syscall_num:tt;) => {
        $(#[$attr])*
        pub async fn $name(
                $($arg1: $type1,)?
                $($arg2: $type2)?
        ) -> $ret_type {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $($arg1)? $($($conv1)*)?,
                    in("x3") $($arg2)? $($($conv2)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            // FIXME: Offer some recourse if the kernel returns null (which indicates running out of
            //        memory).
            unsafe {
                const N: usize = mem::size_of::<$ret_type>();
                let retval_vec = SysCallFuture::from_addr::<N>(addr).await;
                let (retval_arrays, remainder) = retval_vec.as_chunks::<N>();
                assert_eq!(retval_arrays.len(), 1, "syscall {}: return value has the wrong size", stringify!($name));
                assert_eq!(remainder.len(), 0, "syscall {}: return value has the wrong size", stringify!($name));
                mem::transmute(retval_arrays[0])
            }
        }
    };

    (@func $(#[$attr:meta])* pub async fn $name:ident(
        $($arg1:ident: $type1:ty)? $(=> {$($conv1:tt)*})?,
        $($arg2:ident: $type2:ty)? $(=> {$($conv2:tt)*})?,
        $($arg3:ident: $type3:ty)? $(=> {$($conv3:tt)*})?
    ) -> $ret_type:ty => $syscall_num:tt;) => {
        $(#[$attr])*
        pub async fn $name(
                $($arg1: $type1,)?
                $($arg2: $type2,)?
                $($arg3: $type3)?
        ) -> $ret_type {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $($arg1)? $($($conv1)*)?,
                    in("x3") $($arg2)? $($($conv2)*)?,
                    in("x4") $($arg3)? $($($conv3)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            // FIXME: Offer some recourse if the kernel returns null (which indicates running out of
            //        memory).
            unsafe {
                const N: usize = mem::size_of::<$ret_type>();
                let retval_vec = SysCallFuture::from_addr::<N>(addr).await;
                let (retval_arrays, remainder) = retval_vec.as_chunks::<N>();
                assert_eq!(retval_arrays.len(), 1, "syscall {}: return value has the wrong size", stringify!($name));
                assert_eq!(remainder.len(), 0, "syscall {}: return value has the wrong size", stringify!($name));
                mem::transmute(retval_arrays[0])
            }
        }
    };

    (@func $(#[$attr:meta])* pub async fn $name:ident(
        $($arg1:ident: $type1:ty)? $(=> {$($conv1:tt)*})?,
        $($arg2:ident: $type2:ty)? $(=> {$($conv2:tt)*})?,
        $($arg3:ident: $type3:ty)? $(=> {$($conv3:tt)*})?,
        $($arg4:ident: $type4:ty)? $(=> {$($conv4:tt)*})?
    ) -> $ret_type:ty => $syscall_num:tt;) => {
        $(#[$attr])*
        pub async fn $name(
                $($arg1: $type1,)?
                $($arg2: $type2,)?
                $($arg3: $type3,)?
                $($arg4: $type4)?
        ) -> $ret_type {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $($arg1)? $($($conv1)*)?,
                    in("x3") $($arg2)? $($($conv2)*)?,
                    in("x4") $($arg3)? $($($conv3)*)?,
                    in("x5") $($arg4)? $($($conv4)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            // FIXME: Offer some recourse if the kernel returns null (which indicates running out of
            //        memory).
            unsafe {
                const N: usize = mem::size_of::<$ret_type>();
                let retval_vec = SysCallFuture::from_addr::<N>(addr).await;
                let (retval_arrays, remainder) = retval_vec.as_chunks::<N>();
                assert_eq!(retval_arrays.len(), 1, "syscall {}: return value has the wrong size", stringify!($name));
                assert_eq!(remainder.len(), 0, "syscall {}: return value has the wrong size", stringify!($name));
                mem::transmute(retval_arrays[0])
            }
        }
    };

    ($(#[$attr:meta])* pub async fn $name:ident(
        $($($arg:ident: $typ:ty)? $(=> {$($conv:tt)*})?),*
    ) -> $ret_type:ty => $syscall_num:tt;) => {
        define_async_syscall! {
            @func
            $(#[$attr])*
            pub async fn $name($($($arg: $typ)? $(=> {$($conv)*})?),*) -> $ret_type => $syscall_num;
        }
    };
}

macro_rules! define_async_syscalls {
    ($($(#[$attr:meta])* pub async fn $name:ident($($args:tt)*) -> $ret_type:ty => $syscall_num:tt;)+) => {
        $(
            define_async_syscall! {
                $(#[$attr])*
                pub async fn $name($($args)*) -> $ret_type => $syscall_num;
            }
        )+
    };
}

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
///     let child_thread = thread_spawn(entry_point, priority, stack_size);
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
pub fn thread_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "svc 0x0000",
            in("x2") status as usize,
            options(nomem, nostack, preserves_flags, noreturn)
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
pub fn thread_sleep(milliseconds: usize) {
    unsafe {
        asm!(
            "svc 0x0001",
            in("x2") milliseconds,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Spawns a new thread with the given entry point, priority, and stack size.
///
/// Note that this is considered an asynchronous system call for the purposes of [`thread_wait`].
/// The delayed return value is the spawned thread's status code.
///
/// This function is not marked as public because the [`Thread`] type provides a more idiomatic
/// way to call it.
///
/// # Returns
/// An object that represents the new thread.
pub(crate) fn thread_spawn(entry_point: fn(), priority: u8, stack_size: usize) -> Thread {
    let handle: usize;
    unsafe {
        asm!(
            "svc 0x0002",
            in("x2") entry_point as usize,
            in("x3") priority,
            in("x4") stack_size,
            lateout("x0") handle,
            options(nomem, nostack, preserves_flags)
        );
    }
    Thread { handle }
}

/// Waits until an asynchronous system call is finished.
///
/// This function blocks the thread until a system call is complete, allowing other threads to run
/// in the meantime. This may return spurriously; it is up to the caller to determine which system
/// calls, if any, have completed.
///
/// # Returns
/// Nothing. The value is at the given address.
pub fn thread_wait() {
    unsafe {
        asm!(
            "svc 0x0003",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Ends the currently running process with the given status.
///
/// Every thread in the process is immediately terminated.
///
/// # Returns
/// Never returns.
///
/// ```no_run
/// println!("Successful exit from the process");
/// process_exit(0);
/// # #[allow(dead_code)]
/// panic!("This code will never be reached.");
/// ```
pub fn process_exit(status: i32) -> ! {
    unsafe {
        asm!(
            "svc 0x0100",
            in("x2") status as usize,
            options(nomem, nostack, preserves_flags, noreturn)
        );
    }
}

define_async_syscalls! {
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
    pub async fn device_claim(
        name: &str => {as *const str as *const u8 as usize}, => {name.len()}
    ) -> usize => 0x0200;

    /// Frees a block of memory that was allocated by a system call.
    ///
    /// # Panics
    /// If the given address is not the address of the first byte of an allocated block. (The most
    /// likely reason for this to happen is that the memory has already been freed.)
    pub async fn memory_free(
        addr: usize
    ) -> () => 0x0300;

    /// Allocates a new block of virtual memory with the given size and alignment.
    ///
    /// This is only meant for use by the global allocator. Other userspace code should allocate
    /// through that allocator.
    ///
    /// # Returns
    /// The address of the allocated block, or `0` if the allocation failed.
    pub async fn memory_alloc(
        size: usize,
        align: usize
    ) -> usize => 0x0301;

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
    pub async fn memory_alloc_phys(
        size: usize,
        align: usize,
        max_bits: usize
    ) -> VirtPhysAddr => 0x0302;
}


/// Used for packaging a virtual address and a physical address into a single return value.
#[derive(Debug)]
#[repr(C)]
pub struct VirtPhysAddr {
    /// The virtual address.
    pub virt: usize,
    /// The physical address.
    pub phys: usize
}

impl VirtPhysAddr {
    /// Returns a null virtual and physical address.
    pub const fn null() -> Self {
        Self {
            virt: 0,
            phys: 0
        }
    }

    /// Returns `true` if the address is null.
    pub fn is_null(&self) -> bool {
        assert_eq!(self.virt == 0, self.phys == 0);
        self.virt == 0
    }
}
