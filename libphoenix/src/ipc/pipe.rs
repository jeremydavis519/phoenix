/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! A standard pipe to be used for inter-process communication.
//!
//! This communication primitive is based on and fully compatible with the POSIX concept of a pipe:
//! a one-way byte stream that is capable of atomically sending at least 512 bytes at once and
//! allows multiple writers and multiple readers at the same time.

use {
    alloc::{
        alloc::AllocError,
        boxed::Box,
        sync::Arc,
    },
    core::{
        cell::UnsafeCell,
        fmt,
        hint,
        mem::{self, MaybeUninit},
        ptr::{self, addr_of, addr_of_mut},
        sync::atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    crate::{
        ipc::sharing::SharedMemory,
        lock::{RwLock, TryLockError},
        syscall,
    },
};

// A POSIX-defined constant representing the largest number of bytes that is guaranteed to be
// transmissible through a pipe in one atomic operation.
// https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/limits.h.html
const PIPE_BUF: usize = 1024;

// The minimum number of bytes that will actually be allocated to a pipe's buffer. This should be
// more than `PIPE_BUF` so that a writer can write a second atomic batch while the reader is working
// on the first.
#[used]
static MIN_PIPE_SIZE: usize = PIPE_BUF * 2;

mod ffi {
    use super::*;

    /// Allocates a new pipe and returns a pointer to it, or null on error.
    ///
    /// The pipe must be freed by a call to `pipe_free`.
    #[no_mangle]
    extern "C" fn pipe_new() -> *mut Pipe {
        match Pipe::try_new() {
            Ok(pipe) => Box::into_raw(Box::new(pipe)),
            Err(AllocError) => ptr::null_mut(),
        }
    }

    /// Frees the given pipe.
    #[no_mangle]
    extern "C" fn pipe_free(pipe: *mut Pipe) {
        if !pipe.is_null() {
            unsafe { drop(Box::from_raw(pipe)); }
        }
    }
}

/// A pipe that can send serialized data from one process to another.
#[derive(Debug)]
pub struct Pipe {
    buffer: SharedMemory,
}

impl Pipe {
    /// Creates a new pipe.
    ///
    /// # Returns
    /// `Ok`, or `Err(AllocError)` if the pipe couldn't be created for any reason.
    pub fn try_new() -> Result<Self, AllocError> {
        let mut buffer = SharedMemory::try_new(MIN_PIPE_SIZE)?;
        assert!(buffer.len() >= MIN_PIPE_SIZE);
        assert!(MIN_PIPE_SIZE >= mem::size_of::<PipeBuffer>());

        unsafe { PipeBuffer::initialize(buffer.as_raw_slice().as_mut_ptr().cast::<PipeBuffer>()); }

        Ok(Self { buffer })
    }

    /// Adds a writer to the pipe.
    pub fn new_writer(self: &Arc<Self>) -> PipeWriter {
        self.buffer().writers_count.fetch_add(1, Ordering::Release);
        PipeWriter { pipe: self.clone() }
    }

    /// Adds a reader to the pipe.
    pub fn new_reader(self: &Arc<Self>) -> PipeReader {
        self.buffer().readers_count.fetch_add(1, Ordering::Release);
        PipeReader { pipe: self.clone() }
    }

    fn buffer(&self) -> &PipeBuffer {
        unsafe { &*(&*self.buffer as *const _ as *const PipeBuffer) }
    }
}

/// An object that can push data into a pipe.
#[derive(Debug)]
pub struct PipeWriter {
    pipe: Arc<Pipe>,
}

/// An object that can receive data from a pipe.
#[derive(Debug)]
pub struct PipeReader {
    pipe: Arc<Pipe>,
}

impl PipeWriter {
    /// Writes bytes from the given buffer to the pipe.
    ///
    /// This works similarly to POSIX's rules for when `O_NONBLOCK` is set
    /// (https://pubs.opengroup.org/onlinepubs/9699919799/functions/write.html):
    /// * The function does not block the thread.
    /// * If blocking would be necessary, nothing is written and no error occurs.
    /// * If the buffer contains `PIPE_BUF` bytes or fewer, it is written in one atomic operation.
    ///   If there is not enough room in the pipe to write the whole buffer, nothing is written.
    /// * If the buffer contains more than `PIPE_BUF` bytes, as many bytes are written from the
    ///   buffer as the pipe can contain.
    ///
    /// # Returns
    /// `Ok(x)` after writing `x` bytes. `Ok(0)` is possible.
    ///
    /// `Err` if an error occurs. In that case, nothing has been written.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, PipeWriteError> {
        let pipe_buffer = self.pipe.buffer();
        let mut writer_index = match pipe_buffer.writer_index.try_write() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return Ok(0),
        };
        let len = buf.len();
        if len <= PIPE_BUF && pipe_buffer.bytes_free(*writer_index) < len {
            // Not enough room for an atomic write.
            return Ok(0);
        }
        pipe_buffer.write_bytes(buf, &mut *writer_index)
    }

    /// Writes all of the contents of the given buffer to the stream.
    ///
    /// This works similarly to POSIX's rules for when `O_NONBLOCK` is clear
    /// (https://pubs.opengroup.org/onlinepubs/9699919799/functions/write.html):
    /// * If the buffer contains `PIPE_BUF` bytes or fewer, it is written in one atomic operation.
    /// * Otherwise, the write may be interleaved at arbitrary boundaries with those from other
    ///   writers (no guarantees one way or the other).
    /// * The thread will be blocked until either the whole buffer is written or an error occurs.
    ///
    /// # Returns
    /// `Ok` after writing the whole buffer without errors.
    ///
    /// `Err` if an error occurs. In that case, the number of bytes written is 0 if the write was
    /// to be atomic and unspecified otherwise.
    pub fn write_all(&mut self, mut buf: &[u8]) -> Result<(), PipeWriteError> {
        let pipe_buffer = self.pipe.buffer();

        if buf.len() <= PIPE_BUF {
            // Atomic
            let mut writer_index = loop {
                // Wait for enough space in the pipe without forcing other threads to block.
                'wait: loop {
                    for _ in 0 .. 100 {
                        let idx = pipe_buffer.writer_index.read();
                        if pipe_buffer.bytes_free(*idx) >= buf.len() { break 'wait; }
                        hint::spin_loop();
                    }
                    syscall::thread_sleep(0);
                }

                // Confirm that there still is enough space before continuing.
                let writer_index = pipe_buffer.writer_index.write();
                if pipe_buffer.bytes_free(*writer_index) >= buf.len() { break writer_index; }
            };

            assert_eq!(pipe_buffer.write_bytes(buf, &mut *writer_index)?, buf.len());
            return Ok(());
        }

        // Non-atomic
        while !buf.is_empty() {
            let bytes_written = pipe_buffer.write_bytes(buf, &mut *pipe_buffer.writer_index.write())?;
            buf = &buf[bytes_written .. ];
        }

        Ok(())
    }
}

impl PipeReader {
    /// Reads bytes from the pipe into the given buffer.
    ///
    /// This works similarly to POSIX's rules for when `O_NONBLOCK` is set, but with a few
    /// modifications (https://pubs.opengroup.org/onlinepubs/9699919799/functions/read.html):
    /// * The function does not block the thread.
    /// * If the pipe contains some bytes, those bytes are put into the buffer.
    /// * If the pipe is empty and has no writers, an error is returned.
    /// * If the pipe is empty but has a writer, no bytes are read and no error is returned.
    ///
    /// # Returns
    /// `Ok(x)` after reading `x` bytes. `Ok(0)` is possible.
    ///
    /// `Err` if an error prevents reading even one byte.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, PipeReadError> {
        let pipe_buffer = self.pipe.buffer();
        pipe_buffer.read_bytes(buf)
    }

    /// Reads enough bytes from the pipe to fill the given buffer.
    ///
    /// This has no direct analogue in POSIX but can be quite convenient. The thread will be blocked
    /// until the buffer is entirely filled or an error occurs.
    ///
    /// # Returns
    /// `Ok` after writing the whole buffer without errors.
    ///
    /// `Err` if an error occurs. The number of bytes written in this case is unspecified.
    pub fn read_all(&mut self, mut buf: &mut [u8]) -> Result<(), PipeReadError> {
        while buf.len() > 0 {
            for _ in 0 .. 100 {
                let bytes_read = self.read(buf)?;
                buf = &mut buf[bytes_read .. ];
                if buf.len() == 0 { return Ok(()); }
            }
            syscall::thread_sleep(0);
        }
        Ok(())
    }
}

/// An error that can occur when trying to write to a pipe.
#[derive(Debug)]
pub enum PipeWriteError {
    /// The pipe didn't have any readers to receive the written bytes.
    NoReaders,
}

/// An error that can occur when trying to read from a pipe.
#[derive(Debug)]
pub enum PipeReadError {
    /// The pipe was closed and had no more bytes to read.
    PipeClosed,
}

impl fmt::Display for PipeWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoReaders => write!(f, "attempted to write to a pipe with no readers"),
        }
    }
}

impl fmt::Display for PipeReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PipeClosed => write!(f, "attempted to read from a closed pipe"),
        }
    }
}

impl Clone for PipeWriter {
    /// Creates a new writer for the same pipe.
    fn clone(&self) -> Self {
        self.pipe.new_writer()
    }
}

impl Clone for PipeReader {
    /// Creates a new reader for the same pipe.
    fn clone(&self) -> Self {
        self.pipe.new_reader()
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        self.pipe.buffer().writers_count.fetch_sub(1, Ordering::Release);
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        self.pipe.buffer().readers_count.fetch_sub(1, Ordering::Release);
    }
}

#[repr(C)]
struct PipeBuffer {
    readers_count: AtomicU32,
    writers_count: AtomicU32,
    reader_index:  AtomicUsize,
    writer_index:  RwLock<usize>,
    bytes:         UnsafeCell<[u8; 0]>,
}

impl PipeBuffer {
    // Returns the actual length of the `bytes` array of a hypothetical `PipeBuffer`.
    fn bytes_size() -> usize {
        let dummy = MaybeUninit::<Self>::uninit();
        let dummy = dummy.as_ptr();
        let bytes_ptr = unsafe { addr_of!((*dummy).bytes) };
        let offset = unsafe { (*bytes_ptr).get().cast::<u8>().sub_ptr(dummy.cast::<u8>()) };

        // PERF: Cache this value after a single system call.
        let page_size = syscall::memory_page_size();

        MIN_PIPE_SIZE.saturating_add(page_size - 1) / page_size * page_size - offset
    }

    // Initializes a pipe buffer in place.
    unsafe fn initialize(buffer: *mut Self) {
        addr_of_mut!((*buffer).readers_count).write(AtomicU32::new(0));
        addr_of_mut!((*buffer).writers_count).write(AtomicU32::new(0));
        addr_of_mut!((*buffer).reader_index).write(AtomicUsize::new(0));
        addr_of_mut!((*buffer).writer_index).write(RwLock::new(0));
        // `bytes` can be left uninitialized.
    }

    // Returns the number of bytes free, given a particular writer index.
    fn bytes_free(&self, mut writer_index: usize) -> usize {
        let bytes_size = Self::bytes_size();
        let mut reader_index = self.reader_index.load(Ordering::Acquire);

        // If someone corrupted the reader index, try to recover.
        while reader_index >= bytes_size {
            match self.reader_index.compare_exchange_weak(reader_index, 0, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break,
                Err(x) => reader_index = x,
            };
        }

        // A corrupted writer index is easy to fix.
        if writer_index >= bytes_size { writer_index = 0; }

        if reader_index >= writer_index {
            reader_index - writer_index
        } else {
            reader_index + bytes_size - writer_index
        }
    }

    fn write_bytes(&self, mut buf: &[u8], writer_index: &mut usize) -> Result<usize, PipeWriteError> {
        if self.readers_count.load(Ordering::Acquire) == 0 {
            return Err(PipeWriteError::NoReaders);
        }

        let bytes_size = Self::bytes_size();
        let bytes = self.bytes.get().cast::<u8>();

        if *writer_index >= bytes_size { *writer_index = 0; }

        let mut reader_index = self.reader_index.load(Ordering::Acquire);
        if reader_index >= bytes_size { reader_index = 0; }

        if reader_index >= *writer_index {
            // Write all bytes in one chunk.
            let bytes_written = usize::min(buf.len(), reader_index - *writer_index);
            for i in 0 .. bytes_written {
                unsafe { bytes.add(*writer_index + i).write_volatile(buf[i]); }
            }
            *writer_index += bytes_written;
            return Ok(bytes_written);
        }

        // Write bytes up to the array's highest index.
        let first_bytes_written = usize::min(buf.len(), bytes_size - *writer_index);
        for i in 0 .. first_bytes_written {
            unsafe { bytes.add(*writer_index + i).write_volatile(buf[i]); }
        }
        buf = &buf[first_bytes_written .. ];

        if buf.is_empty() {
            *writer_index += first_bytes_written;
            return Ok(first_bytes_written);
        }

        // Write the rest of the bytes at the start of the array.
        let last_bytes_written = usize::min(buf.len(), reader_index);
        for i in 0 .. last_bytes_written {
            unsafe { bytes.add(i).write_volatile(buf[i]); }
        }

        *writer_index = last_bytes_written;
        Ok(first_bytes_written + last_bytes_written)
    }

    fn read_bytes(&self, buf: &mut [u8]) -> Result<usize, PipeReadError> {
        let writer_index = match self.writer_index.try_read() {
            Ok(x) => *x,
            Err(TryLockError::WouldBlock) => return Ok(0), // Can't read anything for now, but no error
        };

        let bytes_size = Self::bytes_size();
        let bytes = self.bytes.get().cast_const().cast::<u8>();

        let mut bytes_read = 0;

        self.reader_index.fetch_update(
            Ordering::AcqRel, 
            Ordering::Acquire, 
            |reader_index| {
                if writer_index >= reader_index {
                    // Read all bytes in one chunk.
                    bytes_read = usize::min(buf.len(), writer_index - reader_index);
                    for i in 0 .. bytes_read {
                        buf[i] = unsafe { bytes.add(reader_index + i).read_volatile() };
                    }
                    return Some(reader_index + bytes_read);
                }

                // Read bytes up to the array's highest index.
                let first_bytes_read = usize::min(buf.len(), bytes_size - reader_index);
                for i in 0 .. first_bytes_read {
                    buf[i] = unsafe { bytes.add(reader_index + i).read_volatile() };
                }
                let buf = &mut buf[first_bytes_read .. ];

                if buf.len() == 0 {
                    bytes_read = first_bytes_read;
                    return Some((reader_index + bytes_read) % bytes_size);
                }

                // Read the rest of the bytes at the start of the array.
                let last_bytes_read = usize::min(buf.len(), writer_index);
                for i in 0 .. last_bytes_read {
                    buf[i] = unsafe { bytes.add(i).read_volatile() };
                }
                bytes_read = first_bytes_read + last_bytes_read;
                Some(last_bytes_read)
            },
        ).unwrap();

        if bytes_read == 0 && self.writers_count.load(Ordering::Acquire) == 0 {
            return Err(PipeReadError::PipeClosed);
        }
        Ok(bytes_read)
    }

    // This function just ensures that the type is FFI-safe. It's necessary because multiple
    // processes will interact with the same object, and each process may have run this code
    // through a different version of rustc (or, if it ever happens, another Rust compiler
    // altogether). Since this type is not exposed in the API, we have to manually expose it like
    // this to get Rust to run its FFI checks.
    extern "C" fn _ffi_safe(self) -> ! {
        unimplemented!("not meant to be called")
    }
}
