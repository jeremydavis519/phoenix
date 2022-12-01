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

#![feature(allocator_api)]
#![feature(coerce_unsized, unsize)]
#![feature(const_type_name)]
#![feature(inline_const)]
#![feature(lang_items)]
#![feature(layout_for_ptr)]
#![feature(panic_info_message)]
#![feature(slice_as_chunks)]

#![cfg_attr(feature = "global-allocator", feature(default_alloc_error_handler))]

extern crate alloc;

// FIXME: This is only here to allow compiling on an x86-64 host.
#[cfg(target_arch = "aarch64")]
pub mod allocator;
// FIXME: This is only here to allow compiling on an x86-64 host.
#[cfg(all(target_arch = "aarch64", not(feature = "kernelspace")))]
mod panic;
pub mod process;
pub mod profiler;
// FIXME: This is only here to allow compiling on an x86-64 host.
#[cfg(target_arch = "aarch64")]
pub mod syscall;
// FIXME: This is only here to allow compiling on an x86-64 host.
#[cfg(target_arch = "aarch64")]
pub mod thread;

#[cfg(target_arch = "aarch64")]
#[cfg(not(any(feature = "no-start", test)))]
#[lang = "start"]
fn lang_start<T: 'static+ProcessReturnValue>(
    main: fn() -> T,
    _argc: isize,
    _argv: *const *const u8
) -> isize {
    let retval = main().retval();
    syscall::process_exit(retval)
}

/// A value that can be returned from `main`.
pub trait ProcessReturnValue {
    /// Converts the given value into something the kernel can understand.
    ///
    /// The value `0` is special in that it indicates a normal termination. All other
    /// values are considered to be abnormal (e.g. caused by some error).
    fn retval(self) -> i32;
}

impl ProcessReturnValue for () {
    fn retval(self) -> i32 { 0 }
}

macro_rules! impl_proc_ret_val {
    ($t:ty) => {
        impl ProcessReturnValue for $t {
            fn retval(self) -> i32 { self.into() }
        }
    };
}

impl_proc_ret_val!(i8);
impl_proc_ret_val!(u8);
impl_proc_ret_val!(i16);
impl_proc_ret_val!(u16);
impl_proc_ret_val!(i32);

/// A macro that returns the name of the containing scope, like C99's `__func__` macro.
///
/// (Based on an answer to [this StackOverflow question].)
/// [this StackOverflow question]: https://stackoverflow.com/questions/38088067/equivalent-of-func-or-function-in-rust
///
/// Note that this relies on unstable Rust functionality, so it may break without warning.
/// Use it only for diagnostic purposes.
#[macro_export]
macro_rules! scope {
    () => {{
        fn f() {}
        const fn type_name_of<T: Copy>(_: T) -> &'static str {
            let name = ::core::any::type_name::<T>();
            unsafe {
                ::core::str::from_utf8_unchecked(::core::slice::from_raw_parts(
                    name.as_ptr(),
                    name.len() - "::f".len(),
                ))
            }
        }
        type_name_of(f)
    }};
}
