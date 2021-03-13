/* Copyright (c) 2019-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module provides a top-level interface with the filesystem. Absolute file paths in this
//! filesystem are written like this:
//! ```text
//! :root/path/to/file
//! ```
//! `:root` is a placeholder for the name of the particular root (see the documentation for `Root`
//! in this module).
//!
//! As in most filesystems, relative paths are also allowed. To form one, simply omit the root.

use alloc::string::String;
use core::fmt;

/// Represents a path in the file system.
#[derive(Debug, Clone, Copy)]
pub struct Path<'a> {
    raw: &'a str
}

impl<'a> Path<'a> {
    /// Makes a new `Path` from the given string.
    pub fn new(path: &'a str) -> Path<'a> {
        Path { raw: path }
    }

    /// Determines whether the path is absolute or relative.
    pub fn absolute(self) -> bool {
        self.raw.starts_with(':')
    }

    /// Splits this path into a file system root and a path relative to that root.
    /// If the path doesn't specify a root, the root is `Root::Relative`.
    pub fn split(self) -> (Root, Path<'a>) {
        if self.absolute() {
            let sep_index = self.raw.find('/');
            let root_len = sep_index.unwrap_or(self.raw.len());
            let stem_start = sep_index.map(|x| x + 1).unwrap_or(self.raw.len()); // Discarding the slash

            let root = &self.raw[ .. root_len];
            let stem = &self.raw[stem_start .. ];

            // TODO: Is there a way to automate the exhaustive matching of roots (apart from
            // `Root::Relative`)? Maybe hash the strings and use `ffi_enum!` to convert from `u64`
            // to an enum variant?
            let root = match root {
                ":host" =>   Root::Host,
                ":initrd" => Root::Initrd,
                _ =>         Root::Unknown
            };

            (root, Path { raw: stem })
        } else {
            (Root::Relative, self)
        }
    }

    /// Converts the `Path` into a UTF-8 string.
    pub fn as_str(&self) -> &'a str {
        self.raw
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl<'a> From<Path<'a>> for String {
    fn from(path: Path) -> String {
        String::from(path.raw)
    }
}

impl<'a> From<&'a str> for Path<'a> {
    fn from(s: &str) -> Path {
        Path::new(s)
    }
}

impl<'a> From<Path<'a>> for &'a str {
    fn from(path: Path) -> &str {
        path.raw
    }
}

/// Represents the root of a file system and contains everything needed to uniquely identify it.
pub enum Root {
    /// `:host`: The host's filesystem when we are running as a guest (e.g. in a virtual machine like Qemu).
    Host,
    /// `:initrd`: The initial RAM disk.
    Initrd,
    /// (no root in the path): The current working directory.
    Relative,
    /// (any root other than those listed here): An unknown root, probably misspelled or nonexistent.
    Unknown
}
