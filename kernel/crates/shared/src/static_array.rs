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

//! This module provides a temporary way to define and initialize large arrays whose elements don't
//! necessarily implement `Copy`.

/// TODO: This is only a stopgap until either the Rust developers or I implement brief static
/// initializers for arbitrarily sized arrays. It should be removed at the first opportunity.
#[macro_export]
macro_rules! array {
    ($x:expr ; 1) => {[
        $x
    ]};

    ($x:expr ; 2) => {[
        $x, $x
    ]};

    ($x:expr ; 4) => {[
        $x, $x, $x, $x
    ]};

    ($x:expr ; 8) => {[
        $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 16) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 32) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 64) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 128) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 256) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 512) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 1024) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 2048) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 4096) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};

    ($x:expr ; 1019) => {[
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x,
        $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x, $x
    ]};
}
