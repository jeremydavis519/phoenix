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

//! A lock that allows access to a single writer or many readers at once.
//!
//! This module's API is based on the Rust standard library's `std::sync::RwLock`.

use {
    core::{
        cell::UnsafeCell,
        hint,
        ops::{Deref, DerefMut},
        sync::atomic::{AtomicUsize, Ordering},
    },
    crate::{
        lock::{TryLockResult, TryLockError},
        syscall,
    },
};

/// A read-write lock.
///
/// This kind of lock is similar to a mutex, except that it allows any number of readers at the
/// same time (up to an unspecified, but large, maximum). A writer, on the other hand, still needs
/// to wait for exclusive access.
#[repr(C)]
#[derive(Debug)]
pub struct RwLock<T: ?Sized> {
    lock:  AtomicUsize,
    value: UnsafeCell<T>,
}

/// An RAII guard used for reading from a read-write lock.
///
/// The lock is released when this object is dropped.
#[derive(Debug)]
pub struct RwLockReadGuard<'a, T: ?Sized>(&'a T);

/// An RAII guard used for reading from and writing to a read-write lock.
///
/// The lock is released when this object is dropped.
#[derive(Debug)]
pub struct RwLockWriteGuard<'a, T: ?Sized>(&'a mut T);

impl<T> RwLock<T> {
    /// Creates a new, unlocked read-write lock.
    pub const fn new(value: T) -> Self {
        Self {
            lock:  AtomicUsize::new(0),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes the lock, returning the previously locked value.
    pub fn into_inner(self) -> T {
        // This is safe because `self` is owned and on the stack. Not even another process
        // interacting through shared memory can have access to it, unless the stack is being
        // shared (and that would be quite concerning in itself).
        self.value.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    const MAX_READERS:      usize = usize::max_value() - 1;
    const WRITER_SIGNATURE: usize = usize::max_value();

    /// Locks the RwLock for reading, blocking if necessary.
    ///
    /// # Returns
    /// A read guard.
    pub fn read(&self) -> RwLockReadGuard<T> {
        loop {
            // Try for a bit in a tight loop in case someone is about to release the lock.
            for _ in 0 .. 100 {
                match self.try_read() {
                    Ok(guard)                     => return guard,
                    Err(TryLockError::WouldBlock) => hint::spin_loop(),
                };
            }
            // Let other threads use the CPU since this is taking a while.
            syscall::thread_sleep(0);
        }
    }

    /// Tries to lock this RwLock for reading.
    ///
    /// # Returns
    /// A read guard, or an error if the lock couldn't be acquired without blocking.
    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<T>> {
        let x = self.lock.load(Ordering::Acquire);

        const { assert!(Self::WRITER_SIGNATURE > Self::MAX_READERS) };
        if x >= Self::MAX_READERS { return Err(TryLockError::WouldBlock); }

        match self.lock.compare_exchange_weak(x, x + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_)  => Ok(RwLockReadGuard(unsafe { &*self.value.get() })),
            Err(_) => Err(TryLockError::WouldBlock),
        }
    }

    /// Locks the RwLock for writing, blocking if necessary.
    ///
    /// # Returns
    /// A write guard.
    pub fn write(&self) -> RwLockWriteGuard<T> {
        loop {
            // Try for a bit in a tight loop in case someone is about to release the lock.
            for _ in 0 .. 100 {
                match self.try_write() {
                    Ok(guard)                     => return guard,
                    Err(TryLockError::WouldBlock) => hint::spin_loop(),
                };
            }
            // Let other threads use the CPU since this is taking a while.
            syscall::thread_sleep(0);
        }
    }

    /// Tries to lock this RwLock for writing.
    ///
    /// # Returns
    /// A write guard, or an error if the lock couldn't be acquired without blocking.
    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<T>> {
        let x = self.lock.load(Ordering::Acquire);

        if x != 0 { return Err(TryLockError::WouldBlock); }

        match self.lock.compare_exchange_weak(x, Self::WRITER_SIGNATURE, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_)  => Ok(RwLockWriteGuard(unsafe { &mut *self.value.get() })),
            Err(_) => Err(TryLockError::WouldBlock),
        }
    }
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
