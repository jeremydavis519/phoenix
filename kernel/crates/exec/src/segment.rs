/* Copyright (c) 2018-2020 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines a file-format-independent representation of a segment of an executable
//! file.

/// A descriptor for a segment of an executable file.
#[derive(Debug)]
pub struct Segment {
    pub(crate) seg_type:    SegmentType,
    pub(crate) flags:       SegmentFlags,
    pub(crate) file_offset: usize,
    pub(crate) vaddr:       usize, // If the file doesn't specify a virtual address, we should decide on one and store it here anyway.
    //pub(crate) paddr:       usize, (ELF includes this field, but we don't use it.)
    pub(crate) file_sz:     usize,
    pub(crate) mem_sz:      usize,
    //pub(crate) align:       usize (ELF includes this field, but we don't use it.)
}

/// Represents the type of an executable file segment.
#[derive(Debug, PartialEq, Eq)]
pub enum SegmentType {
    /// This segment should be loaded into memory.
    Load,
    /// This segment defines the process's stack.
    Stack, // A file can have 0 or 1 stack segment. If 0, the kernel will add a stack while loading.
    /// This segment defines information needed for dynamic linking.
    DLib,
    /// This segment specifies another file to be used as an interpreter for this one.
    Interpreter
}

bitflags! {
    /// Flags that apply to the `Segment` structure.
    pub struct SegmentFlags: u8 {
        /// The segment can be executed.
        const EXECUTABLE = 0x01;
        /// The segment can be read from.
        const READABLE   = 0x02;
        /// The segment can be written to.
        const WRITABLE   = 0x04;
    }
}
