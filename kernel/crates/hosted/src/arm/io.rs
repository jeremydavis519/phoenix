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

//! This module defines the ARM implementation of the I/O parts of the `hosted` API.

use {
    alloc::string::ToString,

    i18n::Text,
    shared::ffi::CStrRef,
    io_crate::{Read, Write, Seek, SeekFrom},
    super::*
};

/// A file in the host's filesystem.
#[derive(Debug)]
pub struct File {
    handle: Field,
    cursor: u64,
    _is_tty: bool,
    _mode: FileMode
}

/// The mode under which a file is to be opened.
#[cfg_attr(target_pointer_width = "32", repr(u32))]
#[cfg_attr(target_pointer_width = "64", repr(u64))]
#[derive(Debug, Clone, Copy)]
pub enum FileMode {
    /// Read-only, textual data.
    ReadText         = 0b0000,
    /// Read-only, binary data.
    ReadBin          = 0b0001,
    /// Read-write, textual data.
    ReadUpdateText   = 0b0010,
    /// Read-write, binary data.
    ReadUpdateBin    = 0b0011,
    /// Write-only, starting from empty, textual data.
    WriteText        = 0b0100,
    /// Write-only, starting from empty, binary data.
    WriteBin         = 0b0101,
    /// Read-write, starting from empty, textual data.
    WriteUpdateText  = 0b0110,
    /// Read-write, starting from empty, binary data.
    WriteUpdateBin   = 0b0111,
    /// Appending at the end, textual data.
    AppendText       = 0b1000,
    /// Appending at the end, binary data.
    AppendBin        = 0b1001,
    /// Read-write, all writes are at the end, textual data.
    AppendUpdateText = 0b1010,
    /// Read-write, all writes are at the end, binary data.
    AppendUpdateBin  = 0b1011
}

impl File {
    /// Attempts to open a file on the host system.
    ///
    /// # Returns
    /// A new `File`, or the value of the C library's `errno` variable.
    pub fn open(path: CStrRef, mode: FileMode) -> Result<File, i64> {
        #[repr(C)]
        struct Params {
            path: Field,
            mode: FileMode,
            path_len: Field
        }
        assert_eq_size!(Params, [Field; 3]);
        let params = Params {
            path: path.as_bytes() as *const [u8] as *const u8 as Field,
            mode,
            path_len: path.len() as Field
        };
        match semihost(Operation::Open, &params as *const _ as Field) {
            -1 => Err(errno()),
            handle => Ok(File {
                handle,
                cursor: 0,
                _is_tty: path == c_str!(":tt"),
                _mode: mode
            })
        }
    }

    /// Returns the number of bytes in this file.
    pub fn len(&self) -> Result<u64, i64> {
        match semihost(Operation::FLen, self.handle as *const Field as Field) {
            -1 => Err(errno()),
            size => Ok(size as u64)
        }
    }

    /// Returns whether this file is actually an interactive terminal.
    pub fn is_tty(&self) -> Result<bool, ()> {
        match semihost(Operation::IsTty, self.handle as *const Field as Field) {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(())
        }
    }

    fn seek_abs(&mut self, pos: u64) -> io_crate::Result<u64> {
        #[repr(C)]
        struct Params {
            handle: Field,
            pos: Field
        }
        assert_eq_size!(Params, [Field; 2]);
        let params = Params {
            handle: self.handle,
            pos: pos as Field
        };
        match semihost(Operation::Seek, &params as *const _ as Field) {
            0 => {
                self.cursor = pos;
                Ok(self.cursor)
            },
            _ => Err(io_crate::ErrorKind::Other.into())
        }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io_crate::Result<usize> {
        #[repr(C)]
        struct Params {
            handle: Field,
            buffer: Field,
            read_size: Field
        }
        assert_eq_size!(Params, [Field; 3]);
        let params = Params {
            handle: self.handle,
            buffer: buf as *mut _ as *mut u8 as Field,
            read_size: buf.len() as Field
        };
        let bytes_left = semihost(Operation::Read, &params as *const _ as Field);
        let bytes_read = buf.len() - bytes_left as usize;
        self.cursor += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io_crate::Result<usize> {
        #[repr(C)]
        struct Params {
            handle: Field,
            buffer: Field,
            write_size: Field
        }
        assert_eq_size!(Params, [Field; 3]);
        let params = Params {
            handle: self.handle,
            buffer: buf as *const _ as *const u8 as Field,
            write_size: buf.len() as Field
        };
        let bytes_left = semihost(Operation::Write, &params as *const _ as Field);
        let bytes_written = buf.len() - bytes_left as usize;
        self.cursor += bytes_written as u64;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> io_crate::Result<()> { Ok(()) }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io_crate::Result<u64> {
        match pos {
            SeekFrom::Start(abs) => self.seek_abs(abs),
            SeekFrom::End(rel) => {
                let size = self.len().map_err(|_errno| io_crate::Error::from(io_crate::ErrorKind::Other))?;
                if rel >= 0 {
                    self.seek_abs(size + rel as u64)
                } else {
                    let neg_rel = (-rel) as u64;
                    if neg_rel > size {
                        Err(io_crate::ErrorKind::InvalidInput.into())
                    } else {
                        self.seek_abs(size - neg_rel)
                    }
                }
            },
            SeekFrom::Current(rel) => {
                if rel >= 0 {
                    self.seek_abs(self.cursor + rel as u64)
                } else {
                    let neg_rel = (-rel) as u64;
                    if neg_rel > self.cursor {
                        Err(io_crate::ErrorKind::InvalidInput.into())
                    } else {
                        self.seek_abs(self.cursor - neg_rel)
                    }
                }
            }
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let _ = self.flush();
        if semihost(Operation::Close, &self.handle as *const _ as Field) == -1 {
            println!("{}", Text::HostedCouldntCloseFile(self.handle.to_string(), errno() as i64));

            // TODO: Remove this old implementation. I'm keeping it around for now because it might
            // be the basis for a good serial-port-free `print!` implementation.
            /* // We can't use any of the `print*!` macros because `std` depends on this crate.
            // Instead, try using the semihosting interface to access the host's `stderr` or
            // `stdout`.
            // TODO: Before trying this, try semihost(Write0) to write to the host debugger console.
            if self.is_tty {
                if writeln!(self, "{}", Text::HostedCouldntCloseFile(self.handle.to_string(), errno() as i64)).is_ok() {
                    return;
                }
            } else if let Ok(mut stderr) = File::open(c_str!(":tt"), FileMode::AppendText) {
                if writeln!(stderr, "{}", Text::HostedCouldntCloseFile(self.handle.to_string(), errno() as i64)).is_ok() {
                    return;
                }
            } else if let Ok(mut stdout) = File::open(c_str!(":tt"), FileMode::WriteText) {
                if writeln!(stdout, "{}", Text::HostedCouldntCloseFile(self.handle.to_string(), errno() as i64)).is_ok() {
                    return;
                }
            } */
        }
    }
}
