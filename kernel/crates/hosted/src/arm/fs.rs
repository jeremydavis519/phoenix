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

//! This module defines the ARM implementation of the file-system-related parts of the `hosted`
//! API, dealing with changing files rather than their contents.

use super::*;

/// Attempts to delete the given file from the host system.
///
/// # Errors
/// Returns a host-specific error code if the deletion fails.
pub fn remove_file(path: &str) -> Result<(), i64> {
    #[repr(C)]
    struct Params {
        path: Field,
        path_len: Field
    }
    assert_eq_size!(Params, [Field; 2]);
    let params = Params {
        path: path.as_bytes() as *const [u8] as *const u8 as Field,
        path_len: path.len() as Field
    };
    match semihost(Operation::Remove, &params as *const _ as Field) {
        0 => Ok(()),
        e => Err(e as i64)
    }
}

/// Attempts to change the given file's name.
///
/// # Errors
/// Returns a host-specific error code if the rename fails.
pub fn rename_file(from: &str, to: &str) -> Result<(), i64> {
    #[repr(C)]
    struct Params {
        from: Field,
        from_len: Field,
        to: Field,
        to_len: Field
    }
    assert_eq_size!(Params, [Field; 4]);
    let params = Params {
        from: from.as_bytes() as *const [u8] as *const u8 as Field,
        from_len: from.len() as Field,
        to: to.as_bytes() as *const [u8] as *const u8 as Field,
        to_len: to.len() as Field
    };
    match semihost(Operation::Rename, &params as *const _ as Field) {
        0 => Ok(()),
        e => Err(e as i64)
    }
}
