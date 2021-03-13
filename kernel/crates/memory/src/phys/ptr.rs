/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines physical pointers and the methods for converting to and from virtual
//! pointers.

use core::cmp;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::ptr;

/// A pointer to a physical memory address. This can be used for memory-mapped I/O, since
/// peripheral devices have no way of knowing their virtual addresses and are often mapped to
/// unchangeable physical addresses. In order for a physical pointer to be used, it must first be
/// cast to a virtual pointer (represented as a raw pointer).
pub struct PhysPtr<T: ?Sized, U: VirtPtrTrait<T>>(U, PhantomData<T>);

/// Groups together the virtual pointer types.
pub trait VirtPtrTrait<T: ?Sized>: Debug+fmt::Pointer+Clone+Copy+PartialEq+Eq+PartialOrd+Ord+Hash {
    /// Returns a null pointer.
    fn null() -> Self where T: Sized;
    /// Returns `true` if this pointer is null.
    fn is_null(self) -> bool;
    /// Casts this pointer to a `usize` representing the start address.
    fn as_usize(self) -> usize;
}
impl<T: ?Sized> VirtPtrTrait<T> for *const T {
    fn null() -> Self where T: Sized { ptr::null() }
    fn is_null(self) -> bool { (self as *const u8).is_null() }
    fn as_usize(self) -> usize { self as *const u8 as usize }
}
impl<T: ?Sized> VirtPtrTrait<T> for *mut T {
    fn null() -> Self where T: Sized { ptr::null_mut() }
    fn is_null(self) -> bool { (self as *const u8).is_null() }
    fn as_usize(self) -> usize { self as *mut u8 as usize }
}

/// Groups together the physical pointer types.
pub trait PhysPtrTrait<T: ?Sized, U: VirtPtrTrait<T>>: Debug+Clone+Copy+PartialEq+Eq {
    /// Returns a null pointer.
    fn null() -> Self where T: Sized;
    /// Returns `true` if this pointer is null.
    fn is_null(self) -> bool { self.raw().is_null() }
    #[doc(hidden)]
    fn raw(self) -> U;
    #[doc(hidden)]
    fn from_raw(raw: U) -> Self;
}
impl<T: ?Sized, U: VirtPtrTrait<T>> PhysPtrTrait<T, U> for PhysPtr<T, U> {
    fn null() -> Self where T: Sized { PhysPtr(U::null(), PhantomData) }
    fn raw(self) -> U { self.0 }
    fn from_raw(raw: U) -> Self { PhysPtr(raw, PhantomData) }
}

// Conversions between slightly different kinds of pointers
impl<T: ?Sized> From<*const T> for PhysPtr<T, *const T> {
    fn from(virt: *const T) -> PhysPtr<T, *const T> {
        PhysPtr::from(virt as *mut T).into()
    }
}
impl<T: ?Sized> From<*mut T> for PhysPtr<T, *mut T> {
    fn from(virt: *mut T) -> PhysPtr<T, *mut T> {
        // TODO: Make this aware of paging.
        PhysPtr(virt, PhantomData)
    }
}
impl<T: ?Sized> From<PhysPtr<T, *mut T>> for PhysPtr<T, *const T> {
    fn from(mutable: PhysPtr<T, *mut T>) -> PhysPtr<T, *const T> {
        PhysPtr::from_raw(mutable.raw() as *const T)
    }
}

// TODO: These should work whenever Rust incorporates trait specialization. Until then, they
// conflict with the blanket `impl<T> From<T> for T`.
/*impl<T: ?Sized, U> From<*const U> for PhysPtr<T, *const T> {
    fn from(virt: *const U) -> PhysPtr<T, *const T> {
        PhysPtr::from(virt as *mut U).into()
    }
}
impl<T: ?Sized, U> From<*mut U> for PhysPtr<T, *mut T> {
    fn from(virt: *mut U) -> PhysPtr<T, *mut T> {
        // TODO: Make this aware of paging.
        PhysPtr(virt as *mut T, PhantomData)
    }
}
impl<T: ?Sized, U: ?Sized> From<PhysPtr<U, *const U>> for PhysPtr<T, *const T> {
    fn from(u: PhysPtr<U, *const U>) -> PhysPtr<T, *const T> {
        PhysPtr::from_raw(u.raw() as *const T)
    }
}
impl<T: ?Sized, U: ?Sized> From<PhysPtr<U, *mut U>> for PhysPtr<T, *mut T> {
    fn from(u: PhysPtr<U, *mut U>) -> PhysPtr<T, *mut T> {
        PhysPtr::from_raw(u.raw() as *mut T)
    }
}
impl<T: ?Sized, U> From<PhysPtr<U, *mut U>> for PhysPtr<T, *const T> {
    fn from(mutable: PhysPtr<U, *mut U>) -> PhysPtr<T, *const T> {
        PhysPtr::from_raw(mutable.raw() as *const U as *const T)
    }
}*/

impl<T: ?Sized> PhysPtr<T, *const T> {
    /// Converts the given physical address to a physical pointer.
    pub fn from_addr_phys(addr: usize) -> PhysPtr<T, *const T>
            where T: Sized {
        PhysPtr(addr as *const T, PhantomData)
    }

    /// Converts this physical pointer to a physical address.
    pub fn as_addr_phys(&self) -> usize {
        self.0 as *const u8 as usize
    }

    /// Converts the given virtual pointer to a physical pointer.
    pub fn from_virt(virt: *const T) -> PhysPtr<T, *const T> {
        PhysPtr(virt, PhantomData)
    }

    /// Converts this physical pointer to a virtual pointer.
    pub fn as_virt(&self) -> Option<*const T> {
        if self.is_null() {
            None
        } else {
            // TODO: Make this aware of paging.
            Some(self.raw())
        }
    }

    /// Converts this physical pointer to a virtual pointer without checking for null. This is safe
    /// because null raw pointers are allowed in Rust, but dereferencing it is unsafe.
    pub fn as_virt_unchecked(&self) -> *const T {
        // TODO: Make this aware of paging.
        self.raw()
    }

    /// This is the equivalent of Rust's raw pointer types' `add` methods. It returns a pointer to
    /// the datum `count * mem::size_of::<T>()` bytes beyond this pointer, in physical memory.
    /// (N.B. `self.add(count).as_virt_unchecked()` might not be the same as
    /// `self.as_virt_unchecked().add(count)`, since physical memory doesn't have to be mapped
    /// contiguously in virtual memory.)
    pub unsafe fn add(&self, count: usize) -> PhysPtr<T, *const T>
            where T: Sized {
        PhysPtr(self.raw().add(count), PhantomData)
    }
}
impl<T: ?Sized> PhysPtr<T, *mut T> {
    /// Converts the given physical address to a physical pointer.
    pub fn from_addr_phys(addr: usize) -> PhysPtr<T, *mut T>
            where T: Sized {
        PhysPtr(addr as *mut T, PhantomData)
    }

    /// Converts this physical pointer to a physical address.
    pub fn as_addr_phys(&self) -> usize {
        self.0 as *mut u8 as usize
    }

    /// Converts the given virtual pointer to a physical pointer.
    pub fn from_virt(virt: *mut T) -> PhysPtr<T, *mut T> {
        PhysPtr(virt, PhantomData)
    }

    /// Converts this physical pointer to a virtual pointer.
    pub fn as_virt(&self) -> Option<*mut T> {
        if self.is_null() {
            None
        } else {
            // TODO: Make this aware of paging.
            Some(self.raw())
        }
    }

    /// Converts this physical pointer to a virtual pointer without checking for null. This is safe
    /// because null raw pointers are allowed in Rust, but dereferencing it is unsafe.
    pub fn as_virt_unchecked(&self) -> *mut T {
        // TODO: Make this aware of paging.
        self.raw()
    }

    /// This is the equivalent of Rust's raw pointer types' `add` methods. It returns a pointer to
    /// the datum `count * mem::size_of::<T>()` bytes beyond this pointer, in physical memory.
    /// (N.B. `self.add(count).as_virt_unchecked()` might not be the same as
    /// `self.as_virt_unchecked().add(count)`, since physical memory doesn't have to be mapped
    /// contiguously in virtual memory.)
    pub unsafe fn add(&self, count: usize) -> PhysPtr<T, *mut T>
            where T: Sized {
        PhysPtr(self.raw().add(count), PhantomData)
    }
}

impl<T: ?Sized, U: VirtPtrTrait<T>> fmt::Pointer for PhysPtr<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.0, f)
    }
}

impl<T: ?Sized, U: VirtPtrTrait<T>> fmt::Debug for PhysPtr<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.raw(), f)
    }
}
impl<T: ?Sized, U: VirtPtrTrait<T>> Clone for PhysPtr<T, U> {
    fn clone(&self) -> Self {
        PhysPtr(self.raw(), PhantomData)
    }
}
impl<T: ?Sized, U: VirtPtrTrait<T>> Copy for PhysPtr<T, U> {}
impl<T: ?Sized, U: VirtPtrTrait<T>> PartialEq for PhysPtr<T, U> {
    fn eq(&self, other: &Self) -> bool {
        &self.raw() == &other.raw()
    }
}
impl<T: ?Sized, U: VirtPtrTrait<T>> Eq for PhysPtr<T, U> {}
impl<T: ?Sized, U: VirtPtrTrait<T>> PartialOrd for PhysPtr<T, U> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&self.raw(), &other.raw())
    }
}
impl<T: ?Sized, U: VirtPtrTrait<T>> Ord for PhysPtr<T, U> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Ord::cmp(&self.raw(), &other.raw())
    }
}
impl<T: ?Sized, U: VirtPtrTrait<T>> Hash for PhysPtr<T, U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw().hash(state)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn need_physptr_tests() {
        // TODO
    }
}
