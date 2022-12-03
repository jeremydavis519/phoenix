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

//! Processes and related concepts.
//!
//! The main type in this module is `Process`, and everything else attaches to it.

use {
    alloc::sync::Arc,
    collections::atomic::AtomicLinkedList,
    exec::ExecImage,
    io::{Read, Seek},
    locks::Semaphore,
    memory::phys::block::BlockMut,
};

/// The combination of an address space and a set of permissions.
///
/// A process is associated with one or more threads, which hold reference-counted pointers to it.
/// Any properties that all of a process's threads share, such as where their page tables are, are
/// stored in the process rather than in each individual thread.
#[derive(Debug)]
pub struct Process<T: Read+Seek> {
    /// The image of the executable file that this process comes from.
    pub exec_image: ExecImage<T>,

    /// A record of all the memory this process might be sharing with another.
    pub shared_memory: Semaphore<AtomicLinkedList<SharedMemory>>,
}

impl<T: Read+Seek> Process<T> {
    /// Creates a new process.
    ///
    /// The new process won't have any threads. Call `Thread::new` to make one.
    pub fn new(exec_image: ExecImage<T>) -> Self {
        Self { exec_image, shared_memory: AtomicLinkedList::new() }
    }
}

/// A record of a block of memory being shared.
///
/// When this object is cloned or dropped, a reference count to the shared memory is updated, and
/// when the last one is dropped, the shared memory is deallocated.
#[derive(Debug, Clone)]
pub struct SharedMemory {
    _block:    Arc<BlockMut<u8>>,
    virt_addr: usize,
}

impl SharedMemory {
    /// Creates a new record of shared memory from the given block and virtual address.
    pub fn new(block: BlockMut<u8>, virt_addr: usize) -> Self {
        Self {
            _block: Arc::new(block),
            virt_addr,
        }
    }

    /// Returns the virtual address of the shared memory in the appropriate address space.
    pub fn virt_addr(&self) -> usize {
        self.virt_addr
    }
}
