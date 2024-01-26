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

//! This module defines what happens when a program linked to `libphoenix` panics.

use {
    core::{
        arch::asm,
        fmt::Write,
        panic::PanicInfo
    },
    crate::syscall
};

#[panic_handler]
#[cold]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    let _ = write!(PanicWriter, "Unexpected error: {}\n", panic_info);
    syscall::process_exit(255) // TODO: Use a named constant for the exit status.
}

// TODO: Get rid of this temporary writer.
struct PanicWriter;

impl core::fmt::Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            temp_putchar(c);
        }
        Ok(())
    }
}

// TODO: This is only a temporary system call. Get rid of it when we have a more robust way to print
//       strings.
fn temp_putchar(c: char) {
    unsafe {
        asm!(
            "svc 0xff00",
            in("x2") c as usize,
            options(nomem, nostack, preserves_flags)
        );
    }
}
