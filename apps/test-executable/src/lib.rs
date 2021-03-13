/* Copyright (c) 2020-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate only exists to test Phoenix's ability to run userspace applications. Once the
//! scheduler is stabilized, this should no longer exist.

#![no_std]
#![feature(asm)]

use core::panic::PanicInfo;

#[no_mangle]
fn _start() -> ! {
    let thread_id: u64;
    unsafe {
        asm!(
            "svc 0x0002", // spawn thread
            in("x2") thread_b, // Entry point
            in("x3") 10,       // Priority
            in("x4") 0x28000,  // Max stack size
            lateout("x0") thread_id,
            options(nomem, preserves_flags, nostack)
        );
    }
    
    if thread_id == 0 {
        for c in "!!!Failed to spawn thread B!!!".chars() {
            putc(c);
        }
        unsafe {
            asm!(
                "svc 0x0000", // terminate thread
                options(nostack, noreturn)
            );
        }
    }

    loop {
        putc('a');
    }
}

fn thread_b() -> ! {
    loop {
        putc('B');
        sleep(0);
    }
}

fn sleep(microseconds: u64) {
    unsafe {
        asm!(
            "svc 0x0001",
            in("x2") microseconds,
            options(nomem, preserves_flags, nostack)
        );
    }
}

fn putc(c: char) {
    unsafe {
        asm!(
            "svc 0xff00",
            in("x2") u64::from(u32::from(c)),
            options(nomem, preserves_flags, nostack)
        );
    }
}

#[panic_handler]
fn panic_handler(_panic_info: &PanicInfo) -> ! {
    unsafe {
        asm!("svc 0xaaaa", options(nostack, noreturn));
    }
}
