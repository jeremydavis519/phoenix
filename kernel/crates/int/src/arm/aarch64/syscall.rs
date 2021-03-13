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

/// This module defines how the Phoenix kernel responds to system calls.

use {
    core::{
        convert::{TryFrom, TryInto},
        time::Duration
    },
    fs::File,
    io::{println, Read},
    scheduler::{Thread, ThreadStatus},
    shared::ffi_enum,
    time::SystemTime,
    super::exceptions::Response
};

pub(crate) fn handle_system_call(
        thread: Option<&mut Thread<File>>,
        syscall: u16,
        args: &[usize; 4],
        result: &mut usize
) -> Response {
    match SystemCall::try_from(syscall) {
        Ok(SystemCall::Thread_Exit)  => thread_exit(thread.map(|t| &*t), args[0]),
        Ok(SystemCall::Thread_Sleep) => thread_sleep(thread, args[0]),
        Ok(SystemCall::Thread_Spawn) => thread_spawn(thread, args[0], args[1] as u8, args[2], result),

        Ok(SystemCall::Process_Exit) => process_exit(thread.map(|t| &*t), args[0]),

        // TODO: Remove all of these temporary system calls.
        Ok(SystemCall::Temp_PutChar) => temp_putchar(args[0]),
        Ok(SystemCall::Temp_GetChar) => temp_getchar(result),

        Err(e) => {
            // TODO: Maybe distinguish between normal termination and a crash.
            // TODO: Send a signal to the thread's parent or something, instead of printing.
            println!("{}", e);
            process_exit(thread.map(|t| &*t), usize::MAX) // TODO: Use a named constant for the failure code.
        }
    }
}

ffi_enum! {
    #[repr(u16)]
    #[allow(non_camel_case_types)]
    enum SystemCall {
        Thread_Exit  = 0x0000,
        Thread_Sleep = 0x0001,
        Thread_Spawn = 0x0002,

        Process_Exit = 0x0100,

        Temp_PutChar = 0xff00,
        Temp_GetChar = 0xff01
    }
}

// Terminates the current thread, returning to the kernel's state from before the thread started
// running.
fn thread_exit(thread: Option<&Thread<File>>, result: usize) -> Response {
    assert!(!thread.is_none(), "attempted to terminate a kernel thread");
    if result != 0 {
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
        handle: &mut usize
) -> Response {
    let parent_thread = thread.expect("attempted to spawn a new kernel thread");
    let entry_point = usize::try_from(entry_point).unwrap();
    // TODO: A priority of 0 should maybe mean real-time (i.e. cooperative scheduling only). We'll
    // need to adjust the load-balancing logic to account for that.
    if priority == 0 {
        priority = 1;
    }
    *handle = scheduler::spawn_thread(parent_thread.exec_image.clone(), entry_point, max_stack_size, priority)
        .unwrap_or(0);
    Response::eret()
}


// Terminates the process containing the current thread, thereby terminating every thread in that
// process.
fn process_exit(thread: Option<&Thread<File>>, result: usize) -> Response {
    assert!(!thread.is_none(), "attempted to terminate a kernel thread's process");
    if result != 0 {
        // FIXME: Handle process return values.
        unimplemented!("process return value");
    }
    // FIXME: Exit the process, not just the current thread.
    Response::leave_userspace(ThreadStatus::Terminated)
}


// TODO: Remove these temporary system calls.
fn temp_putchar(c: usize) -> Response {
    io::print!("{}", char::try_from(c as u32).unwrap_or('?'));
    Response::eret()
}
fn temp_getchar(c: &mut usize) -> Response {
    let mut stdin = io::STDIN.try_lock().unwrap();
    let mut buffer = [0u8; 1];
    stdin.read_exact(&mut buffer[ .. ])
        .expect("error reading from standard input");
    io::print!("{}", core::str::from_utf8(&buffer).unwrap_or("?"));
    *c = buffer[0].into();
    Response::eret()
}
