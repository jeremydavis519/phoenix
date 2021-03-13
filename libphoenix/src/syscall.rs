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

use super::thread::Thread;

/*macro_rules! define_async_syscall {
    (@func $name:ident() -> $ret_type:ty { $syscall_num:tt }) => {
        pub fn $name<'future>() -> Future<'future, $ret_type> {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            Future { promised: unsafe { &mut *(addr as *mut _) } }
        }
    };

    (@func $name:ident(
            $arg1:ident: $type1:ty $(=> $($conv1:tt)*)?
    ) -> $ret_type:ty { $syscall_num:tt }) => {
        pub fn $name<'future>(
                $arg1: $type1
        ) -> Future<'future, $ret_type> {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $arg1 $($($conv1:tt)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            Future { promised: unsafe { &mut *(addr as *mut _) } }
        }
    };

    (@func $name:ident(
        $arg1:ident: $type1:ty $(=> $($conv1:tt)*)?,
        $arg2:ident: $type2:ty $(=> $($conv2:tt)*)?
    ) -> $ret_type:ty { $syscall_num:tt }) => {
        pub fn $name<'future>(
                $arg1: $type1,
                $arg2: $type2
        ) -> Future<'future, $ret_type> {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $arg1 $($($conv1:tt)*)?,
                    in("x3") $arg2 $($($conv2:tt)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            Future { promised: unsafe { &mut *(addr as *mut _) } }
        }
    };

    (@func $name:ident(
        $arg1:ident: $type1:ty $(=> $($conv1:tt)*)?,
        $arg2:ident: $type2:ty $(=> $($conv2:tt)*)?,
        $arg3:ident: $type3:ty $(=> $($conv3:tt)*)?
    ) -> $ret_type:ty { $syscall_num:tt }) => {
        pub fn $name<'future>(
                $arg1: $type1,
                $arg2: $type2,
                $arg3: $type3
        ) -> Future<'future, $ret_type> {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $arg1 $($($conv1:tt)*)?,
                    in("x3") $arg2 $($($conv2:tt)*)?,
                    in("x4") $arg3 $($($conv3:tt)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            Future { promised: unsafe { &mut *(addr as *mut _) } }
        }
    };

    (@func $name:ident(
        $arg1:ident: $type1:ty $(=> $($conv1:tt)*)?,
        $arg2:ident: $type2:ty $(=> $($conv2:tt)*)?,
        $arg3:ident: $type3:ty $(=> $($conv3:tt)*)?,
        $arg4:ident: $type4:ty $(=> $($conv4:tt)*)?
    ) -> $ret_type:ty { $syscall_num:tt }) => {
        pub fn $name<'future>(
                $arg1: $type1,
                $arg2: $type2,
                $arg3: $type3,
                $arg4: $type4
        ) -> Future<'future, $ret_type> {
            let addr: usize;
            unsafe {
                asm!(
                    concat!("svc ", $syscall_num),
                    in("x2") $arg1 $($($conv1:tt)*)?,
                    in("x3") $arg2 $($($conv2:tt)*)?,
                    in("x4") $arg3 $($($conv3:tt)*)?,
                    in("x5") $arg4 $($($conv4:tt)*)?,
                    lateout("x0") addr,
                    options(nomem, preserves_flags)
                );
            }
            Future { promised: unsafe { &mut *(addr as *mut _) } }
        }
    };

    ($name:ident($($arg:ident: $typ:ty $(=> $($conv:tt)*)?),*) -> $ret_type:ty { $syscall_num:tt }) => {
        define_async_syscall! {
            @func
            $name($($arg: $typ $(=> $($conv)*)?),*) -> $ret_type { $syscall_num }
        }
    };
}

macro_rules! define_async_syscalls {
    ($($(#[$attr:meta])* $name:ident($($args:tt)*) -> $ret_type:ty { $syscall_num:tt })+) => {
        $(
            $(#[$attr])*
            define_async_syscall!($name($($args)*) -> $ret_type { $syscall_num });
        )+
    };
}*/

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
///     panic!("This code will never be reached.");
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
/// This function is not marked as public because the [`Thread`] class provides a more idiomatic
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

/// Blocks on the result of an asynchronous system call.
///
/// This function should *not* be made public because it is an implementation detail of the
/// [`Future`] type. It would only provide a worse, unsafe interface to something that can already
/// be done in safe code.
///
/// # Returns
/// Nothing. The value is at the given address.
pub(crate) fn future_block(promised_value_addr: usize) {
    unsafe {
        asm!(
            "svc 0x0200",
            in("x2") promised_value_addr,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/*define_async_syscalls! {
}*/
