/* Copyright (c) 2022-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines all the standard methods of inter-process communication (IPC) that Phoenix
//! supports.

use {
    alloc::vec::Vec,
    crate::{
        lock::RwLock,
        serde::{Serializer, Deserializer, Serialize, Deserialize, SerializeError, DeserializeError},
    },
};

pub mod pipe;
pub mod sharing;

pub use pipe::*;
pub use sharing::*;

#[cfg(not(feature = "no-start"))]
pub(crate) static INHERITED_FILE_DESCRIPTORS: RwLock<Vec<FileDescriptor>> = RwLock::new(Vec::new());

/// A file descriptor of any kind.
#[allow(missing_docs)]
pub enum FileDescriptor {
    PipeReader(PipeReader),
    PipeWriter(PipeWriter),
}

impl Serialize for FileDescriptor {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        match self {
            Self::PipeReader(pipe_reader) => pipe_reader.serialize(s),
            Self::PipeWriter(pipe_writer) => pipe_writer.serialize(s),
        }
    }
}

impl Deserialize for FileDescriptor {
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError>
        where Self: Sized {
        if let Ok(pipe_reader) = PipeReader::deserialize(d) { return Ok(Self::PipeReader(pipe_reader)); }
        if let Ok(pipe_writer) = PipeWriter::deserialize(d) { return Ok(Self::PipeWriter(pipe_writer)); }
        Err(DeserializeError)
    }
}
