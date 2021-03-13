/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines various helpful abstractions to use when interfacing with things outside of
//! Rust.

use {
    core::{
        fmt,
        marker::PhantomData,
        slice,
        str
    },
    error::Error
};

/// Wraps enum definitions with specific `#[repr]`s, adding a way to convert from an integer to the
/// enum type.
#[macro_export]
macro_rules! ffi_enum {
    ($(
        #[repr( $base_type:tt )]
        $(#[$post_attr:meta])*
        $vis:vis enum $enum:ident {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident $(= $val:expr)?
            ),* $(,)?
        }
      )*) => {
        $(
            #[repr($base_type)]
            $(#[$post_attr])*
            $vis enum $enum {
                $(
                    $(#[$variant_attr])*
                    $variant $(= $val)?
                ),*
            }
            impl core::convert::TryFrom<$base_type> for $enum {
                type Error = $crate::ffi::InvalidVariantError<$base_type>;

                fn try_from(value: $base_type) -> Result<$enum, Self::Error> {
                    match value {
                        $(x if x == $enum::$variant as $base_type => Ok($enum::$variant),)*
                        value => Err($crate::ffi::InvalidVariantError::new(stringify!($enum), value))
                    }
                }
            }
            impl From<$enum> for $base_type {
                fn from(value: $enum) -> $base_type {
                    value as $base_type
                }
            }
        )*
    };
}

/// Represents an error that can occur when trying to convert an integer to a variant of an enum
/// defined by the `ffi_enum` macro.
// This definition currently excludes u128-backed enums. We probably won't need any of those,
// though.
#[derive(Debug)]
pub struct InvalidVariantError<T: Into<Int128>+Copy+core::fmt::Debug> {
    enum_type: &'static str,
    value: T
}

impl<T: Into<Int128>+Copy+core::fmt::Debug> InvalidVariantError<T> {
    /// Makes a new instance of the error for the given enum type and integer value. (The meaning
    /// is that this integer cannot be converted to the enum type.)
    pub fn new(enum_type: &'static str, value: T) -> InvalidVariantError<T> {
        InvalidVariantError { enum_type, value }
    }
}

impl<T: Into<Int128>+Copy+core::fmt::Debug> Error for InvalidVariantError<T> {}

impl<T: Into<Int128>+Copy+core::fmt::Debug> fmt::Display for InvalidVariantError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "attempted to construct a variant of enum `{}` from invalid numeric representation {}", self.enum_type, self.value.into())
    }
}

// A signed or unsigned 128-bit number. This exists only to make `InvalidVariantError` work with
// `#[repr(u128)]` enums.
#[doc(hidden)]
#[derive(Debug)]
pub enum Int128 {
    Signed(i128),
    Unsigned(u128)
}
impl fmt::Display for Int128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Signed(x) => x.fmt(f),
            Self::Unsigned(x) => x.fmt(f)
        }
    }
}
macro_rules! impl_into_int128 {
    ($type:ty, $sign:ident) => {
        impl From<$type> for Int128 {
            fn from(x: $type) -> Int128 {
                Int128::$sign(x.into())
            }
        }
    };
}
impl_into_int128!(i8, Signed);
impl_into_int128!(u8, Unsigned);
impl_into_int128!(i16, Signed);
impl_into_int128!(u16, Unsigned);
impl_into_int128!(i32, Signed);
impl_into_int128!(u32, Unsigned);
impl_into_int128!(i64, Signed);
impl_into_int128!(u64, Unsigned);
impl_into_int128!(i128, Signed);
impl_into_int128!(u128, Unsigned);

/// Represents a big-endian integer. It needs to be converted to a regular integer before being
/// used as anything other than a bit pattern.
#[repr(transparent)]
pub struct Be<T: PrimitiveEndian>(T);

/// Represents a little-endian integer. It needs to be converted to a regular integer before
/// being used as anything other than a bit pattern.
#[repr(transparent)]
pub struct Le<T: PrimitiveEndian>(T);

/// Represents any newtype for a particular representation of an integer, such as the `Be` and `Le`
/// types.
pub trait Endian {
    /// The native-endian equivalent of the type that implements this trait.
    type Primitive;
    /// Converts from native-endian to this type's representation.
    fn from_native(x: Self::Primitive) -> Self;
    /// Converts from this type's representation to native-endian.
    fn into_native(self) -> Self::Primitive;
}

impl<T: PrimitiveEndian> Endian for Be<T> {
    type Primitive = T;
    fn from_native(x: T) -> Self { Self(x.to_be()) }
    fn into_native(self) -> T { T::from_be(self.0) }
}

impl<T: PrimitiveEndian> Endian for Le<T> {
    type Primitive = T;
    fn from_native(x: T) -> Self { Self(x.to_le()) }
    fn into_native(self) -> T { T::from_le(self.0) }
}

macro_rules! impl_endian_traits {
    ( $type:ident ) => {
        impl<T: PrimitiveEndian+Clone> Clone for $type::<T> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl<T: PrimitiveEndian+Copy> Copy for $type::<T> {}

        impl<T: PrimitiveEndian+Default> Default for $type::<T> {
            fn default() -> Self {
                Self::from_native(T::default())
            }
        }

        impl<T: PrimitiveEndian+fmt::Debug> fmt::Debug for $type::<T> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

impl_endian_traits!(Be);
impl_endian_traits!(Le);

/// Represents any type that can be converted between big- and little-endian forms. The `Be` and
/// `Le` types are defined only as wrappers for types that implement this trait.
pub trait PrimitiveEndian {
    /// Converts from big-endian to native-endian.
    fn from_be(x: Self) -> Self;
    /// Converts from little-endian to native-endian.
    fn from_le(x: Self) -> Self;
    /// Converts from native-endian to big-endian.
    fn to_be(self) -> Self;
    /// Converts from native-endian to little-endian.
    fn to_le(self) -> Self;
}

macro_rules! impl_endian {
    ($type:ty) => {
        impl PrimitiveEndian for $type {
            fn from_be(x: Self) -> Self { Self::from_be(x) }
            fn from_le(x: Self) -> Self { Self::from_le(x) }
            fn to_be(self) -> Self { self.to_be() }
            fn to_le(self) -> Self { self.to_le() }
        }
    };
}

impl_endian!(u8);
impl_endian!(u16);
impl_endian!(u32);
impl_endian!(u64);
impl_endian!(u128);
impl_endian!(usize);
impl_endian!(i8);
impl_endian!(i16);
impl_endian!(i32);
impl_endian!(i64);
impl_endian!(i128);
impl_endian!(isize);

/// Represents a reference to a C string, similar to `&str` but stored in memory as an array of
/// bytes that ends at the first null byte.
#[repr(transparent)]
pub struct CStrRef<'a>(&'a u8);

/// Converts a literal `&'static str` into a `CStrRef` to cut down a bit on repetitive code and ugly
/// null terminators in string literals.
#[macro_export]
macro_rules! c_str {
    ($str:expr) => {
        unsafe { $crate::ffi::CStrRef::from_null_terminated_slice_unchecked(concat!($str, "\0").as_bytes()) }
    }
}

impl<'a> CStrRef<'a> {
    /// Converts the given raw pointer to a C string reference.
    ///
    /// # Safety
    /// This function causes the raw pointer to be dereferenced, which is undefined behavior if the
    /// memory in question is uninitialized, if the pointer is null, or if a mutable reference to
    /// the memory already exists. The string must be initialized up to the first zero byte (the
    /// null terminator).
    ///
    /// The safe version of this function is `from_null_terminated_slice`.
    pub unsafe fn from_ptr(ptr: *const u8) -> CStrRef<'a> {
        CStrRef(&*ptr)
    }

    /// Converts a null-terminated byte slice into a C string reference.
    ///
    /// # Returns
    /// `None` if the slice is not null-terminated or has a null byte somewhere other than at the
    /// end, else `Some`.
    pub const fn from_null_terminated_slice(slice: &[u8]) -> Option<CStrRef> {
        if slice.len() == 0 || slice[slice.len() - 1] != 0 {
            return None;
        }
        let mut i = 0;
        while i < slice.len() - 1 {
            if slice[i] == 0 {
                return None;
            }
            i += 1;
        }
        Some(unsafe { Self::from_null_terminated_slice_unchecked(slice) })
    }

    /// Converts a null-terminated byte slice into a C string reference, skipping the check done by
    /// `from_null_terminated_slice`.
    ///
    /// # Safety
    /// It is undefined behavior to call this in any situation in which
    /// `from_null_terminated_slice` would return `None`.
    pub const unsafe fn from_null_terminated_slice_unchecked(slice: &[u8]) -> CStrRef {
        CStrRef(&slice[0])
    }

    /// Converts the C string reference into a Rust string reference.
    ///
    /// # Returns
    /// `Err(core::str::Utf8Error)` if the C string is not valid UTF-8 text.
    pub fn as_str(&self) -> Result<&str, str::Utf8Error> { str::from_utf8(self.as_bytes()) }

    /// Converts the C string reference into a Rust string reference, with the length capped to
    /// `max_len`. If the string's length is less than `max_len`, this is equivalent to `as_str`.
    ///
    /// # Returns
    /// `Err(core::str::Utf8Error)` if the first `max_len` bytes of the string are not valid UTF-8
    /// text.
    pub fn as_str_capped(&self, max_len: usize) -> Result<&str, str::Utf8Error> { str::from_utf8(self.as_bytes_capped(max_len)) }

    /// Performs the same conversion as `as_str`, skipping the safety check.
    ///
    /// # Safety
    /// It is undefined behavior to call this if `as_str` would return an error.
    pub unsafe fn as_str_unchecked(&self) -> &str { str::from_utf8_unchecked(self.as_bytes()) }

    /// Performs the same conversion as `as_str_capped`, skipping the safety check.
    ///
    /// # Safety
    /// It is undefined behavior to call this if `as_str_capped` would return an error.
    pub unsafe fn as_str_capped_unchecked(&self, max_len: usize) -> &str { str::from_utf8_unchecked(self.as_bytes_capped(max_len)) }

    /// Converts the C string into a slice of bytes. The null terminator is not included in the
    /// slice.
    pub fn as_bytes(&self) -> &[u8] { unsafe { slice::from_raw_parts(self.0, self.len()) } }

    /// Converts the C string into a slice of bytes, including the null terminator as the last
    /// element.
    pub fn as_bytes_null_terminated(&self) -> &[u8] { unsafe { slice::from_raw_parts(self.0, self.len() + 1) } }

    /// Converts the first `max_len` bytes of the C string into a slice of bytes. If the string's
    /// length is less than `max_len`, this is equivalent to `as_bytes`.
    pub fn as_bytes_capped(&self, max_len: usize) -> &[u8] { unsafe { slice::from_raw_parts(self.0, self.len_capped(max_len)) } }

    /// Returns an iterator over the bytes in this C string, excluding the null terminator.
    pub const fn iter(&'a self) -> CStrBytes<'a> {
        CStrBytes { cursor: self.0, _phantom: PhantomData }
    }

    /// Returns the number of bytes in this C string, excluding the null terminator.
    pub const fn len(&self) -> usize {
        // This does the same as `self.iter().count()`, but that isn't `const`.
        let mut i = 0;
        loop {
            if unsafe { *(self.0 as *const u8).add(i) } == 0 {
                return i;
            }
            i += 1;
        }
    }

    /// Returns the number of bytes in this C string, excluding the null terminator, or `max_len`,
    /// whichever is smaller. This should be used for efficiency when you don't care about
    /// differentiating strings above a certain length, and for security when you're not sure
    /// whether a string is properly null-terminated.
    pub const fn len_capped(&self, max_len: usize) -> usize {
        // This does the same as `self.iter().take(max_len).count()`, but that isn't `const`.
        let mut i = 0;
        while i < max_len {
            if unsafe { *(self.0 as *const u8).add(i) } == 0 {
                return i;
            }
            i += 1;
        }
        max_len
    }
}

impl fmt::Debug for CStrRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}
impl fmt::Display for CStrRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.iter()
            .map(|&c| write!(f, "{}", char::from(c)))
            .find(|r| r.is_err())
            .unwrap_or(Ok(()))
    }
}

impl PartialEq for CStrRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.iter()
            .eq(other.iter())
    }
}

/// An iterator over the individual bytes in a C string, not including the null terminator.
#[derive(Debug)]
pub struct CStrBytes<'a> {
    cursor: *const u8,
    _phantom: PhantomData<&'a u8>
}

impl<'a> Iterator for CStrBytes<'a> {
    type Item = &'a u8;

    fn next(&mut self) -> Option<Self::Item> {
        let byte = unsafe { &*self.cursor };
        if *byte == 0 {
            None
        } else {
            self.cursor = unsafe { self.cursor.add(1) };
            Some(byte)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add more tests.

    mod c_strings {
        use super::*;

        #[test]
        fn good_strings() {
            let good_strings = [
                "\0",
                "foo\0",
                "string\0",
                "This is a sentence.\0",
                "bar\0",
                "$ymb0|5\0",
                "\x01\x02\x03\0"
            ];
            for &string in good_strings.iter() {
                let c_string = CStrRef::from_null_terminated_slice(string.as_bytes());
                assert!(c_string.is_some());
                let c_string = c_string.unwrap();
                assert_eq!(c_string.len(), string.len() - 1);
                unsafe {
                    assert_eq!(c_string, CStrRef::from_null_terminated_slice_unchecked(string.as_bytes()));
                    assert_eq!(c_string, CStrRef::from_ptr(string as *const str as *const u8));
                }
            }
        }
        
        #[test]
        fn bad_strings() {
            let bad_strings = [
                "",
                "foo",
                "string",
                "This is a sentence.",
                "bar",
                "$ymb0|5",
                "\x01\x02\x03",
                "\0foo\0",
                "str\0ing\0",
                "This is\0a sentence.",
                "$\0ymb0|5\0",
                "\x01\x02\0\x03\0"
            ];
            for &string in bad_strings.iter() {
                let c_string = CStrRef::from_null_terminated_slice(string.as_bytes());
                assert!(c_string.is_none());
                // Even if we can't safely make the string, it should still behave as it would in C.
                if let Some(len) = string.find('\0') {
                    unsafe {
                        let c_string = CStrRef::from_null_terminated_slice_unchecked(string.as_bytes());
                        assert_eq!(c_string, CStrRef::from_ptr(string as *const str as *const u8));
                        assert_eq!(c_string.len(), len);
                    }
                }
            }
        }
    }
}

