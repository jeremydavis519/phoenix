/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the kernel's behavior when it panics.

#![cfg(not(feature = "unit-test"))]

use {
    core::panic::PanicInfo,
    i18n::Text
};

// Defines the panic runtime. This is a very simple runtime, since a kernel panic should always be
// treated as an unrecoverable error. We just print a message and abort.
#[panic_handler]
#[cold]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    println!("{}", Text::UnexpectedKernelError(panic_info));
    // TODO: Can we manage to get any kind of backtrace here? Or maybe a core dump?
    unsafe { hang() }
}

/// Does nothing forever. It's private and `unsafe` because the kernel generally shouldn't do that
/// unless it's panicking.
unsafe fn hang() -> ! {
    // Since we're supposed to be doing nothing, we shouldn't handle any interrupts. That could
    // cause further corruption.
    shared::disable_interrupts();
    loop { shared::wait_for_interrupt(); }
}
