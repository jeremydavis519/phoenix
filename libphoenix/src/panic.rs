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

use core::panic::PanicInfo;

// FIXME: #[panic_handler], and not `pub`
//        Using this attribute here causes a linker error. If it's not fixed soon, we should try to
//        find a minimal example and submit an issue to the Rust repository.
#[doc(hidden)] // TODO: Not needed once this becomes private.
#[cold]
pub fn panic_handler(_: &PanicInfo) -> ! {
    // FIXME: Print some debug information and close the program using a defined system call.
    unsafe {
        asm!(
            "svc 0xaaaa", // Undefined system call
            options(nomem, nostack, preserves_flags, noreturn)
        );
    }
}
