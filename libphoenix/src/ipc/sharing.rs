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

//! Raw shared memory.
//!
//! This module provides access to the inter-process communication primitive of shared memory. It
//! should probably never be used directly but is available for use if the more abstract types of
//! IPC, like pipes, are insufficient.

use {
    alloc::alloc::AllocError,
    core::{
        convert::TryFrom,
        ops::Deref,
        ptr,
        sync::atomic::AtomicU8,
    },
    crate::{
        serde::{Serialize, Deserialize, Serializer, Deserializer, SerializeError, DeserializeError, serialize_object},
        syscall,
    },
};

/// An RAII-enabled representation of a shared memory block.
#[derive(Debug)]
pub struct SharedMemory {
    bytes: *mut [AtomicU8],
}

impl SharedMemory {
    /// Allocates a new block of shared memory.
    ///
    /// See the documentation on [`memory_alloc_shared`] for more details.
    ///
    /// # Returns
    /// `Ok`, or `Err(AllocError)` if the block couldn't be allocated for any reason.
    pub fn try_new(len: usize) -> Result<Self, AllocError> {
        let ptr = syscall::memory_alloc_shared(len);
        if ptr.is_null() {
            return Err(AllocError);
        }

        let bytes = ptr::slice_from_raw_parts_mut(ptr.cast::<AtomicU8>(), len);
        for i in 0 .. len {
            unsafe { bytes.get_unchecked_mut(i).write(AtomicU8::new(0)); }
        }

        Ok(Self { bytes })
    }

    /// Returns the shared memory as a raw byte slice.
    pub fn as_raw_slice(&mut self) -> *mut [AtomicU8] {
        self.bytes
    }
}

impl Deref for SharedMemory {
    type Target = [AtomicU8];

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.bytes }
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe { syscall::memory_free(self.bytes.as_mut_ptr().cast()); }
    }
}

impl Serialize for SharedMemory {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        let addr = u64::try_from(self.bytes.cast::<AtomicU8>().addr())
            .map_err(|_| SerializeError)?;
        let len = u64::try_from(self.bytes.len())
            .map_err(|_| SerializeError)?;

        serialize_object!(s, {
            "addr" => |s| s.u64(addr),
            "len"  => |s| s.u64(len),
        })
    }
}

impl Deserialize for SharedMemory {
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<(Self, usize), DeserializeError> {
        let mut addr = None;
        let mut len = None;

        let ((), serialized_len) = d.object(|field_name, mut deserializer| {
            let field_len;
            match field_name {
                "addr" => {
                    if addr.is_some() { return Err(DeserializeError); }
                    let (val, val_len) = deserializer.u64()?;
                    addr = Some(val);
                    field_len = val_len;
                },
                "len" => {
                    if len.is_some() { return Err(DeserializeError); }
                    let (val, val_len) = deserializer.u64()?;
                    len = Some(val);
                    field_len = val_len;
                },
                _ => return Err(DeserializeError),
            };
            Ok(field_len)
        })?;

        let Some(addr) = addr else { return Err(DeserializeError) };
        let Some(len) = len else { return Err(DeserializeError) };

        let addr = usize::try_from(addr).map_err(|_| DeserializeError)?;
        let len = usize::try_from(len).map_err(|_| DeserializeError)?;

        let ptr = syscall::memory_access_shared(addr, len);

        let len = if ptr.is_null() { 0 } else { len };

        Ok((Self { bytes: ptr::slice_from_raw_parts_mut(ptr.cast::<AtomicU8>(), len) }, serialized_len))
    }
}
