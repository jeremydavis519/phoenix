/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate provides an API for interacting with the host when running in an emulator or virtual
//! machine like Bochs or QEMU. When running on real hardware, everything in here is equivalent to
//! `Err(NotHosted)`.

#![no_std]

#![deny(warnings, missing_docs)]

extern crate alloc;
#[cfg_attr(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"), macro_use)] extern crate io as io_crate;

#[cfg(not(target_arch = "x86_64"))]
#[macro_use] extern crate bitflags;
#[cfg(not(target_arch = "x86_64"))]
#[macro_use] extern crate static_assertions;
#[cfg(not(target_arch = "x86_64"))]
#[macro_use] extern crate shared;

#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    mod arm;
#[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    pub use self::arm::*;
#[cfg(feature = "unit-test")] mod shim;
#[cfg(feature = "unit-test")] pub use self::shim::*;
