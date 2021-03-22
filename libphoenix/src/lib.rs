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

//! This crate is a sort of "standard library" of functions and types that are generally useful to
//! all userspace programs written for the Phoenix operating system.

#![no_std]
#![deny(warnings, missing_docs)]

#![feature(asm)]

extern crate alloc;

pub mod allocator;
pub mod future;
pub mod panic; // FIXME: Not `pub` (blocked on making `panic::panic_handler` private).
pub mod process;
pub mod syscall;
pub mod thread;

/// Defines the entry point of the program.
///
/// Since Phoenix isn't officially supported by Rust, the runtime has a really hard time understanding
/// how to compile and link an executable file (i.e. a `bin` crate). This macro automates some of the
/// workarounds we have to use to get it to work. Just write a `main` function like you normally would
/// and wrap it in an invocation of this macro, after applying the `start` feature to the crate.
///
/// You will also have to pass `-C link-arg=--entry=main` to rustc so the linker will be able to find
/// the entry point. That should be automatic, but for some reason it isn't. Probably because rustc
/// thinks we're compiling for an architecture without an operating system.
///
/// # Example
/// ```
/// #![feature(start)]
/// phoenix_main! {
///     fn main() {
///         println!("Hello, world!");
///     }
/// }
/// ```
#[macro_export]
macro_rules! phoenix_main {
    // TODO: Allow signatures with argc and argv.

    ($vis:vis fn main() $(-> ())? { $($body:tt)* }) => {
        $vis fn main() { $($body)* }

        #[start]
        fn start(_argc: isize, _argv: *const *const u8) -> isize {
            main();
            $crate::syscall::process_exit($crate::process::ExitCode::Success as i32);
        }

        #[panic_handler]
        fn panic_handler(p: &core::panic::PanicInfo) -> ! {
            $crate::panic::panic_handler(p)
        }
    };

    ($vis:vis fn main() -> Result<(), $error:ty> { $($body:tt)* }) => {
        $vis fn main() { $($body)* }

        #[start]
        fn start(_argc: isize, _argv: *const *const u8) -> isize {
            let status = match main() {
                Ok(()) => $crate::process::ExitCode::Success,
                Err(e) => {
                    eprintln!("{:?}", e);
                    $crate::process::ExitCode::Failure
                }
            };
            $crate::syscall::process_exit(status as i32);
        }

        #[panic_handler]
        fn panic_handler(p: &core::panic::PanicInfo) -> ! {
            $crate::panic::panic_handler(p)
        }
    };
}
