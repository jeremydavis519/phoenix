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

//! This crate defines a simple InitRD format and the interface used to read it. The InitRD is
//! constructed as part of the crate's build process, stored as Rust data structures, and exposed in
//! this module as `ROOT`.

#![no_std]

#![deny(warnings, missing_docs)]

include!(concat!(env!("OUT_DIR"), "/contents.rs"));

/// Represents a directory and allows its contents to be accessed.
#[derive(Debug)]
pub struct Directory<'a> {
    /// The name of the directory within its parent directory.
    pub name: &'a str,
    /// All of the subdirectories contained within this one.
    pub subdirs: &'a [Directory<'a>],
    /// All the files contained directly within this directory.
    pub files: &'a [File<'a>]
}

/// Represents a file and allows its contents to be accessed.
#[derive(Debug)]
pub struct File<'a> {
    /// The filename.
    pub name: &'a str,
    /// The contents of the file.
    pub contents: &'a [u8]
}

impl<'a> Directory<'a> {
    /// Searches for and returns the `Directory` corresponding to the given path, using
    /// `self` as the root. The path should *not* start or end with a slash. Only forward
    /// slashes separate path elements, not backslashes.
    /// 
    /// # Returns
    /// A `Directory`, or `None` if no such directory exists.
    pub fn find_dir(&self, path: &str) -> Option<&Directory> {
        if path.is_empty() {
            return Some(self);
        }

        let pieces = path.split('/');
        let mut dir = self;
        for name in pieces.filter(|p| p.len() > 0) { // Search pieces of the path one at a time
            let mut found = false;
            for subdir in dir.subdirs { // Look for a matching subdirectory
                if subdir.name == name {
                    dir = subdir;
                    found = true;
                    break;
                }
            }
            if !found {
                return None; // No matching subdirectory
            }
        }
        Some(dir)
    }

    /// Searches for and returns the `File` corresponding to the given path, using
    /// `self` as the root. The path should *not* start or end with a slash. Only forward
    /// slashes separate path elements, not backslashes.
    /// 
    /// # Returns
    /// A `File`, or `None` if no such file exists.
    pub fn find_file(&self, path: &str) -> Option<&File> {
        let (name, dir) = match path.rfind('/') {
            Some(last_sep_index) => {
                if let Some(dir) = self.find_dir(&path[ .. last_sep_index]) {
                    let name = &path[last_sep_index + 1 .. ];
                    (name, dir)
                } else {
                    return None; // Containing directory doesn't exist
                }
            },
            None => (path, self)
        };
        for file in dir.files {
            if file.name == name {
                return Some(file);
            }
        }
        None // File not found
    }
}

// TODO: Add tests.
