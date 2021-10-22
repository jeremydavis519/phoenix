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

//! This module defines an interface for dealing with threads in Phoenix.

use super::syscall;

/// An object that represents a thread.
#[derive(Debug)]
pub struct Thread {
    pub(crate) _handle: usize
}

impl Thread {
    /// Spawns a new thread with the given entry point, priority, and stack size.
    ///
    /// # Returns
    /// An object that represents the new thread.
    ///
    /// # Example
    /// ```no_run
    /// fn main() {
    ///     let mut child_thread = Thread::spawn(do_work, 10, 0x10000);
    ///     let status = child_thread.join();
    ///     assert_eq!(status, 0);
    /// }
    ///
    /// fn do_work() {
    ///     println!("Hello from the child thread!");
    /// }
    /// ```
    pub fn spawn(entry_point: fn(), priority: u8, stack_size: usize) -> Thread {
        syscall::thread_spawn(entry_point, priority, stack_size)
    }

    /// Blocks until the given thread completes.
    ///
    /// # Returns
    /// The thread's exit status.
    /// ```no_run
    /// fn main() {
    ///     let mut child_thread = Thread::spawn(do_work, 10, 0x10000);
    ///     let status = child_thread.join();
    ///     assert_eq!(status, 42);
    /// }
    ///
    /// fn do_work() {
    ///     syscall::thread_exit(42);
    /// }
    /// ```
    pub fn join(&mut self) -> i32 {
        // TODO
        unimplemented!("Thread::join");
    }
}
