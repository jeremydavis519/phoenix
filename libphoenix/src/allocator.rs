/* Copyright (c) 2021-2024 Jeremy Davis (jeremydavis519@gmail.com)
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
use {
    core::ffi::{c_void, c_int},
    crate::posix::errno::Errno,
};

#[cfg(feature = "global-allocator")]
extern "C" {
    #[thread_local]
    static mut errno: c_int;
}

// From the POSIX descriptions of `malloc` and `realloc` (both quotes are identical):
// "The pointer returned if the allocation succeeds shall be suitably aligned so that it may be
// assigned to a pointer to any type of object and then used to access such an object in the
// space allocated ...."
const ALIGNMENT_FOR_ANYTHING: usize = 16;

// https://pubs.opengroup.org/onlinepubs/9699919799/functions/malloc.html
#[cfg(feature = "global-allocator")]
#[no_mangle]
unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let Ok(layout) = Layout::from_size_align(size, ALIGNMENT_FOR_ANYTHING) else {
        errno = Errno::ENOMEM.into();
        return ptr::null_mut();
    };
    let ptr = Allocator.alloc(layout);
    if ptr.is_null() {
        errno = Errno::ENOMEM.into();
    }
    ptr.cast::<c_void>()
}

// https://pubs.opengroup.org/onlinepubs/9699919799/functions/free.html
#[cfg(feature = "global-allocator")]
#[no_mangle]
unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() { return; }

    let prefix_size = (mem::size_of::<AllocPrefix>() + (ALIGNMENT_FOR_ANYTHING - 1)) & !(ALIGNMENT_FOR_ANYTHING);
    let prefix = ptr.cast::<u8>().sub(prefix_size).cast::<AllocPrefix>();
    let Ok(layout) = Layout::from_size_align((*prefix).size, ALIGNMENT_FOR_ANYTHING) else { return };
    Allocator.dealloc(ptr.cast::<u8>(), layout);
}

// https://pubs.opengroup.org/onlinepubs/9699919799/functions/realloc.html
#[cfg(feature = "global-allocator")]
#[no_mangle]
unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() { return malloc(size); }

    let prefix_size = (mem::size_of::<AllocPrefix>() + (ALIGNMENT_FOR_ANYTHING - 1)) & !(ALIGNMENT_FOR_ANYTHING);
    let prefix = ptr.cast::<u8>().sub(prefix_size).cast::<AllocPrefix>();
    let Ok(layout) = Layout::from_size_align((*prefix).size, ALIGNMENT_FOR_ANYTHING) else {
        errno = Errno::ENOMEM.into();
        return ptr::null_mut();
    };
    let ptr = Allocator.realloc(ptr.cast::<u8>(), layout, size);
    if ptr.is_null() {
        errno = Errno::ENOMEM.into();
    }
    ptr.cast::<c_void>()
}

#[cfg(feature = "global-allocator")]
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

/// A memory allocator capable of getting new memory from the kernel.
#[derive(Debug, Clone, Copy)]
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
            unsafe { addr.virt.write(MaybeUninit::uninit()); }
            Ok(PhysBox {
                ptr: addr.virt.cast::<MaybeUninit<T>>(),
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
            let slice = ptr::slice_from_raw_parts_mut(addr.virt.cast::<MaybeUninit<T>>(), len);
            for i in 0 .. len {
                unsafe { slice.get_unchecked_mut(i).write(MaybeUninit::uninit()); }
            }
            Ok(PhysBox {
                ptr:  slice,
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
            let slice = ptr::slice_from_raw_parts_mut(addr.virt, size);
            for i in 0 .. size {
                unsafe { slice.get_unchecked_mut(i).write(MaybeUninit::uninit()); }
            }
            Ok(PhysBox {
                ptr: slice,
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
        let prefix_size = (mem::size_of::<AllocPrefix>() + (layout.align() - 1)) & !(layout.align() - 1);
        let ptr = syscall::memory_alloc(prefix_size + layout.size(), layout.align());
        (*ptr.cast::<MaybeUninit<AllocPrefix>>()).write(AllocPrefix { size: layout.size() }); // Record the size for future calls to libc's `free` and `realloc`.
        ptr.cast::<u8>().add(prefix_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let prefix_size = (mem::size_of::<AllocPrefix>() + (layout.align() - 1)) & !(layout.align() - 1);
        let ptr = ptr.cast::<MaybeUninit<u8>>().sub(prefix_size);
        ptr.cast::<AllocPrefix>().drop_in_place();
        syscall::memory_free(ptr);
    }

    // TODO: Write a more efficient implementation of `GlobalAlloc::realloc`.
}

#[derive(Debug)]
struct AllocPrefix {
    size: usize,
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
