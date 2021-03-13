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

use {
    io::{Read, Write, Seek, SeekFrom},
    shared::ffi::CStrRef,
    super::shim_panic
};

#[derive(Debug)]
pub struct File;
#[derive(Debug)]
pub enum FileMode {
    ReadText         = 0b0000,
    ReadBin          = 0b0001,
    ReadUpdateText   = 0b0010,
    ReadUpdateBin    = 0b0011,
    WriteText        = 0b0100,
    WriteBin         = 0b0101,
    WriteUpdateText  = 0b0110,
    WriteUpdateBin   = 0b0111,
    AppendText       = 0b1000,
    AppendBin        = 0b1001,
    AppendUpdateText = 0b1010,
    AppendUpdateBin  = 0b1011
}

impl File {
    pub fn open(_: CStrRef, _: FileMode) -> Result<File, i64> { shim_panic("io::File::open"); }
    pub fn len(&self) -> Result<u64, i64> { shim_panic("io::File::len"); }
    pub fn is_tty(&self) -> Result<bool, ()> { shim_panic("io::File::is_tty"); }
}

impl Read for File {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> { shim_panic("io::File::read"); }
}

impl Write for File {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> { shim_panic("io::File::write"); }
    fn flush(&mut self) -> io::Result<()> { shim_panic("io::File::flush"); }
}

impl Seek for File {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> { shim_panic("io::File::seek"); }
}
