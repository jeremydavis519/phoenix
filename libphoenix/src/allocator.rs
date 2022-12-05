/* Copyright (c) 2021-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines the standard allocator for Phoenix programs.
//!
//! This sets the global allocator, since most programs shouldn't need to define their own
//! allocators, but it's not required. If a program does not want the global allocator, it should
//! disable the `global-allocator` feature (and every library that depends on `libphoenix` should
//! expose that feature in its own `Cargo.toml`, as follows:
//!
//! ```toml
//! [features]
//! global-allocator = ["libphoenix/global-allocator"]
//! ```

use {
    alloc::alloc::{Layout, GlobalAlloc, AllocError},
    core::{
        marker::Unsize,
        mem::{self, MaybeUninit},
        ops::{CoerceUnsized, Deref, DerefMut},
        ptr,
    },
    crate::syscall,
};

#[cfg(feature = "global-allocator")]
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

/// A memory allocator capable of getting new memory from the kernel.
#[derive(Debug)]
pub struct Allocator;

impl Allocator {
    /// Allocates a new `T` object on the heap in a way that makes its physical address visible.
    ///
    /// See [`memory_alloc_phys`](crate::syscall::memory_alloc_phys) for more details.
    pub fn malloc_phys<T>(&self, max_bits: usize) -> Result<PhysBox<MaybeUninit<T>>, AllocError> {
        let addr = syscall::memory_alloc_phys(
            mem::size_of::<T>(),
            mem::align_of::<T>(),
            max_bits,
        );

        if addr.is_null() {
            Err(AllocError)
        } else {
            Ok(PhysBox {
                ptr: addr.virt as *mut MaybeUninit<T>,
                phys: addr.phys,
            })
        }
    }

    /// Allocates a new `[T]` array on the heap in a way that makes its physical address visible.
    ///
    /// See [`memory_alloc_phys`](crate::syscall::memory_alloc_phys) for more details.
    pub fn malloc_phys_array<T>(&self, len: usize, max_bits: usize) -> Result<PhysBox<[MaybeUninit<T>]>, AllocError> {
        let addr = syscall::memory_alloc_phys(
            mem::size_of::<T>() * len,
            mem::align_of::<T>(),
            max_bits,
        );

        if addr.is_null() {
            Err(AllocError)
        } else {
            Ok(PhysBox {
                ptr: ptr::slice_from_raw_parts_mut(addr.virt as *mut MaybeUninit<T>, len),
                phys: addr.phys,
            })
        }
    }

    /// Allocates a new array of bytes on the heap in a way that makes its physical address visible.
    ///
    /// See [`memory_alloc_phys`](crate::syscall::memory_alloc_phys) for more details.
    pub fn malloc_phys_bytes(&self, size: usize, align: usize, max_bits: usize)
            -> Result<PhysBox<[MaybeUninit<u8>]>, AllocError> {
        let addr = syscall::memory_alloc_phys(
            size,
            align,
            max_bits,
        );

        if addr.is_null() {
            Err(AllocError)
        } else {
            Ok(PhysBox {
                ptr: ptr::slice_from_raw_parts_mut(addr.virt as *mut MaybeUninit<u8>, size),
                phys: addr.phys,
            })
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // FIXME: This is extremely wasteful, as the kernel can't give us anything smaller than
        // a page, and it can also take a while. Instead, allocate a buffer from the kernel and use
        // that for multiple allocations until it's full.
        let addr = syscall::memory_alloc(
            layout.size(),
            layout.align(),
        );
        addr as *mut u8
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        syscall::memory_free(ptr as usize);
    }
}


/// A smart pointer that remembers the physical address of its referent in addition to its virtual
/// address. This is intended for use in drivers, which sometimes need access to physical memory
/// addresses.
#[derive(Debug)]
pub struct PhysBox<T: ?Sized> {
    ptr: *mut T,
    phys: usize,
}

impl<T> PhysBox<T> {
    /// Allocates a box and places the given value inside it. Analogous to `Box::new`.
    pub fn new(value: T) -> Self {
        let mut phys_box = Allocator.malloc_phys::<T>(mem::size_of::<usize>() * 8)
            .expect("failed to allocate a PhysBox");
        phys_box.write(value);
        PhysBox::assume_init(phys_box)
    }
}

impl<T: ?Sized> PhysBox<T> {
    /// Returns the physical address of the object that this box contains.
    pub fn addr_phys(&self) -> usize {
        self.phys
    }

    /// Consumes the box without freeing any memory and returns a raw pointer to the boxed value and
    /// its physical address. These should be passed to [`from_raw`] later in order to avoid a
    /// memory leak.
    pub fn into_raw(boxed: Self) -> (*mut T, usize) {
        let raw = (boxed.ptr, boxed.phys);
        mem::forget(boxed);
        raw
    }

    /// Takes a raw pointer and physical address previously returned by [`into_raw`] and converts
    /// them back into a box. It is undefined behavior to dereference the raw pointer after calling
    /// this method.
    pub fn from_raw(ptr: *mut T, phys: usize) -> Self {
        Self { ptr, phys }
    }
}

impl<T> PhysBox<MaybeUninit<T>> {
    /// Unwraps the `MaybeUninit` in the same manner as `MaybeUninit::assume_init`.
    pub fn assume_init(boxed: Self) -> PhysBox<T> {
        let (ptr, phys) = PhysBox::into_raw(boxed);
        PhysBox::from_raw(ptr as *mut T, phys)
    }
}

impl<T> PhysBox<[MaybeUninit<T>]> {
    /// Unwraps all the `MaybeUninit` values in the slice in the same manner as `MaybeUninit::assume_init`.
    pub fn slice_assume_init(boxed: Self) -> PhysBox<[T]> {
        let (ptr, phys) = PhysBox::into_raw(boxed);
        PhysBox::from_raw(ptr as *mut [T], phys)
    }

    /// Initializes each value in the slice using the given function.
    ///
    /// The argument to the function is the index of the element being initialized.
    pub fn init_each<F: Fn(usize) -> T>(mut boxed: Self, f: F) -> PhysBox<[T]> {
        for (i, value) in boxed.iter_mut().enumerate() {
            value.write(f(i));
        }
        Self::slice_assume_init(boxed)
    }
}

impl<T: ?Sized+Unsize<U>, U: ?Sized> CoerceUnsized<PhysBox<U>> for PhysBox<T> {}

impl<T: ?Sized> Deref for PhysBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: ?Sized> DerefMut for PhysBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T: ?Sized> Drop for PhysBox<T> {
    fn drop(&mut self) {
        unsafe {
            Allocator.dealloc(self.ptr as *mut u8, Layout::for_value_raw(self.ptr));
        }
    }
}
