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

//! Serialization and deserialization of arbitrary data.
//!
//! This interface is necessary for IPC, since that always goes through an FFI boundary. Any type
//! that can be serialized and deserialized must implement the [`Serialize`] and [`Deserialize`]
//! traits.
//!
//! [`Serialize`]: trait.Serialize.html
//! [`Deserialize`]: trait.Deserialize.html

use {
    alloc::{
        alloc::AllocError,
        rc::Rc,
        string::String,
        sync::Arc,
        vec::Vec,
    },
    core::{
        any::Any,
        error,
        fmt,
        ops::{Deref, DerefMut},
    },
};

/// An abstraction over any serialization implementation.
pub trait Serializer {
    /// The type that is used to serialize a single field during a call to the [`object`] function.
    ///
    /// [`object`]: #tymethod.object.html
    type FieldSerializer: Serializer;

    /// The type that represents an iterator over the serializers for all of the fields in an
    /// object during a call to the [`object`] function.
    ///
    /// [`object`]: #tymethod.object.html
    type FieldSerializers<S: DerefMut<Target = Self::FieldSerializer>>: IntoIterator<Item = S>;

    /// Serializes a string.
    fn string(&mut self, value: &str) -> Result<(), SerializeError>;
    /// Serializes a boolean value.
    fn bool(&mut self, value: bool) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn i8(&mut self, value: i8) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn i16(&mut self, value: i16) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn i32(&mut self, value: i32) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn i64(&mut self, value: i64) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn i128(&mut self, value: i128) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn u8(&mut self, value: u8) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn u16(&mut self, value: u16) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn u32(&mut self, value: u32) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn u64(&mut self, value: u64) -> Result<(), SerializeError>;
    /// Serializes an integer.
    fn u128(&mut self, value: u128) -> Result<(), SerializeError>;

    /// Serializes a list of serializable values.
    fn list<T: Serialize, I: IntoIterator<Item = T>>(&mut self, values: I) -> Result<(), SerializeError>;

    /// Serializes an object with named fields.
    ///
    /// The serializers are given to the `serialize` callback function in the same order as the
    /// corresponding names appear in `field_names`.
    ///
    /// The arguments are designed for use with the [`fields`] macro. If you decide to write them
    /// by hand, your code is likely to become quite messy.
    ///
    /// This function returns an error if the callback returns an error, and it may also return an
    /// error even if the callback succeeds.
    ///
    /// [`fields`]: ../macro.fields.html
    fn object<S, N, F>(&mut self, field_names: N, serialize: F)
        -> Result<(), SerializeError>
        where S: Deref<Target = str>,
              N: IntoIterator<Item = S>,
              F: FnOnce(Self::FieldSerializers<&mut Self::FieldSerializer>) -> Result<(), SerializeError>;

    /// Serializes a serializable value.
    fn serialize<T: Serialize>(&mut self, value: T) -> Result<(), SerializeError> {
        value.serialize(self)
    }

    /// Serializes a serializable value, but only once.
    ///
    /// If the same value (by pointer equality) is passed to this function more than once, the
    /// first call serializes the value and returns a unique index by which it can be referenced
    /// after serialization. Then, all subsequent calls with the same value look up and return this
    /// index without serializing a new copy of the value.
    ///
    /// Returned indices are not guaranteed to be consecutive, only unique.
    fn serialize_once<T: Serialize, P: Deref<Target = T>>(&mut self, value: P) -> Result<u32, SerializeError>;
}

/// Converts a comma-separated list of fields into the correct type for the [`Serializer::object`]
/// function.
///
/// Each field is written as a string literal, a wide arrow, and then a function that accepts a
/// `&mut [Serializer]` and uses it to serialize the field's value.
///
/// This macro smooths over the rough edges that can be found in that API. It can be used like
/// this:
/// ```
/// fn foo<S: Serializer>(s: &mut Serializer) -> Result<(), SerializeError> {
///     serialize_object!(s {
///         "bar" => |s| s.u32(42),
///         "baz" => |s| s.string("value"),
///     })
/// }
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __serde_serialize_object__ {
    (
        $s:expr, {
            $($name:expr => |$serializer:ident $(: $t:ty)?| $serialize:expr),* $(,)?
        }
    ) => {
        $crate::serde::Serializer::object(
            $s,
            ::core::iter::empty()$(.chain(::core::iter::once($name)))*,
            |serializers| {
                let mut serializers = serializers.into_iter();
                $(match serializers.next() {
                    Some(s) => {
                        // A little convoluted, but Rust's type inference can't handle real
                        // closures in this context.
                        let $serializer$(: $t)? = s;
                        $serialize?;
                    },
                    None => {
                        // Not enough serializers were provided.
                        return ::core::result::Result::Err($crate::serde::SerializeError);
                    },
                };)*
                match serializers.next() {
                    Some(_) => {
                        // Too many serializers were provided.
                        return ::core::result::Result::Err($crate::serde::SerializeError);
                    },
                    None => ::core::result::Result::Ok(()),
                }
            },
        )
    };
}

#[doc(inline)]
pub use __serde_serialize_object__ as serialize_object;

/// An abstraction over any deserialization implementation.
pub trait Deserializer {
    /// The type that is used to deserialize a single field during a call to the [`object`]
    /// function.
    ///
    /// [`object`]: #tymethod.object.html
    type FieldDeserializer: Deserializer;

    /// The type that is used to deserialize a single field during a call to the
    /// [`deserialize_once`] function.
    ///
    /// [`deserialize_once`]: #tymethod.deserialize_once.html
    type OnceDeserializer: Deserializer;

    /// Deserializes a string.
    fn string(&mut self) -> Result<String, DeserializeError>;
    /// Deserializes a boolean value.
    fn bool(&mut self) -> Result<bool, DeserializeError>;
    /// Deserializes an integer.
    fn i8(&mut self) -> Result<i8, DeserializeError>;
    /// Deserializes an integer.
    fn i16(&mut self) -> Result<i16, DeserializeError>;
    /// Deserializes an integer.
    fn i32(&mut self) -> Result<i32, DeserializeError>;
    /// Deserializes an integer.
    fn i64(&mut self) -> Result<i64, DeserializeError>;
    /// Deserializes an integer.
    fn i128(&mut self) -> Result<i128, DeserializeError>;
    /// Deserializes an integer.
    fn u8(&mut self) -> Result<u8, DeserializeError>;
    /// Deserializes an integer.
    fn u16(&mut self) -> Result<u16, DeserializeError>;
    /// Deserializes an integer.
    fn u32(&mut self) -> Result<u32, DeserializeError>;
    /// Deserializes an integer.
    fn u64(&mut self) -> Result<u64, DeserializeError>;
    /// Deserializes an integer.
    fn u128(&mut self) -> Result<u128, DeserializeError>;

    /// Deserializes a vector of deserializable values.
    fn vec<T: Deserialize>(&mut self) -> Result<Vec<T>, DeserializeError>;

    /// Deserializes an object with named fields.
    ///
    /// The callback function takes the name of a field and a deserializer for the field's value.
    /// It is called once for every field in the object, potentially more than once per field name
    /// (if the same name appears more than once in the object).
    ///
    /// Any error returned by the callback causes this function to return an error. In that case,
    /// it is unspecified which, if any, other fields will be passed to the callback as well. This
    /// function may also return an error even if every callback succeeds.
    fn object<F: FnMut(&str, Self::FieldDeserializer) -> Result<(), DeserializeError>>(&mut self, deserialize_field: F)
        -> Result<(), DeserializeError>;

    /// Deserializes a value of the given type.
    fn deserialize<T: Deserialize>(&mut self) -> Result<T, DeserializeError> {
        T::deserialize(self)
    }

    /// Looks up a value that was serialized with [`serialize_once`].
    ///
    /// This function is intended to be used to deserialize a value that has multiple owners, like
    /// a value of type `Rc` or `Arc`.
    ///
    /// If the value was indeed serialized and this is the first attempt to deserialize it, this
    /// calls the given `deserialize` function, then stores the result for later. Subsequent calls
    /// to this function with the same index simply return the stored result.
    ///
    /// If the value was not serialized (i.e. the index doesn't match any serialized value), this
    /// function returns an error.
    ///
    /// [`serialize_once`]: ../trait.Serializer.html#tymethod.serialize_once
    fn deserialize_once<T: Any, F: FnOnce(Self::OnceDeserializer) -> T>(
        &mut self,
        index: u32,
        deserialize: F,
    ) -> Result<T, DeserializeError>;
}

impl<T: Serializer + ?Sized> Serializer for &mut T {
    type FieldSerializer = T::FieldSerializer;
    type FieldSerializers<S: DerefMut<Target = Self::FieldSerializer>> = T::FieldSerializers<S>;

    fn string(&mut self, value: &str) -> Result<(), SerializeError> { T::string(*self, value) }
    fn bool(&mut self, value: bool) -> Result<(), SerializeError> { T::bool(*self, value) }
    fn i8(&mut self, value: i8) -> Result<(), SerializeError> { T::i8(*self, value) }
    fn i16(&mut self, value: i16) -> Result<(), SerializeError> { T::i16(*self, value) }
    fn i32(&mut self, value: i32) -> Result<(), SerializeError> { T::i32(*self, value) }
    fn i64(&mut self, value: i64) -> Result<(), SerializeError> { T::i64(*self, value) }
    fn i128(&mut self, value: i128) -> Result<(), SerializeError> { T::i128(*self, value) }
    fn u8(&mut self, value: u8) -> Result<(), SerializeError> { T::u8(*self, value) }
    fn u16(&mut self, value: u16) -> Result<(), SerializeError> { T::u16(*self, value) }
    fn u32(&mut self, value: u32) -> Result<(), SerializeError> { T::u32(*self, value) }
    fn u64(&mut self, value: u64) -> Result<(), SerializeError> { T::u64(*self, value) }
    fn u128(&mut self, value: u128) -> Result<(), SerializeError> { T::u128(*self, value) }
    fn list<U: Serialize, I: IntoIterator<Item = U>>(&mut self, values: I) -> Result<(), SerializeError> { T::list(*self, values) }
    fn object<S, N, F>(&mut self, field_names: N, serialize: F)
            -> Result<(), SerializeError>
            where S: Deref<Target = str>,
                  N: IntoIterator<Item = S>,
                  F: FnOnce(Self::FieldSerializers<&mut Self::FieldSerializer>) -> Result<(), SerializeError> {
        T::object(*self, field_names, serialize)
    }
    fn serialize<U: Serialize>(&mut self, value: U) -> Result<(), SerializeError> { T::serialize(*self, value) }
    fn serialize_once<U: Serialize, P: Deref<Target = U>>(&mut self, value: P) -> Result<u32, SerializeError> { T::serialize_once(*self, value) }
}

impl<T: Deserializer + ?Sized> Deserializer for &mut T {
    type FieldDeserializer = T::FieldDeserializer;
    type OnceDeserializer = T::OnceDeserializer;

    fn string(&mut self) -> Result<String, DeserializeError> { T::string(*self) }
    fn bool(&mut self) -> Result<bool, DeserializeError> { T::bool(*self) }
    fn i8(&mut self) -> Result<i8, DeserializeError> { T::i8(*self) }
    fn i16(&mut self) -> Result<i16, DeserializeError> { T::i16(*self) }
    fn i32(&mut self) -> Result<i32, DeserializeError> { T::i32(*self) }
    fn i64(&mut self) -> Result<i64, DeserializeError> { T::i64(*self) }
    fn i128(&mut self) -> Result<i128, DeserializeError> { T::i128(*self) }
    fn u8(&mut self) -> Result<u8, DeserializeError> { T::u8(*self) }
    fn u16(&mut self) -> Result<u16, DeserializeError> { T::u16(*self) }
    fn u32(&mut self) -> Result<u32, DeserializeError> { T::u32(*self) }
    fn u64(&mut self) -> Result<u64, DeserializeError> { T::u64(*self) }
    fn u128(&mut self) -> Result<u128, DeserializeError> { T::u128(*self) }
    fn vec<U: Deserialize>(&mut self) -> Result<Vec<U>, DeserializeError> { T::vec(*self) }
    fn object<F: FnMut(&str, Self::FieldDeserializer) -> Result<(), DeserializeError>>(&mut self, deserialize_field: F)
            -> Result<(), DeserializeError> {
        T::object(*self, deserialize_field)
    }
    fn deserialize<U: Deserialize>(&mut self) -> Result<U, DeserializeError> { T::deserialize(*self) }
    fn deserialize_once<U: Any, F: FnOnce(Self::OnceDeserializer) -> U>(
            &mut self,
            index: u32,
            deserialize: F,
        ) -> Result<U, DeserializeError> {
        T::deserialize_once(*self, index, deserialize)
    }
}

/// An interface for serializing any type that can be safely serialized.
pub trait Serialize {
    /// Serializes this object by passing it into the given serializer.
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError>;
}

/// An interface for deserializing a type.
pub trait Deserialize {
    /// Deserializes an object from the given deserializer.
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError>
        where Self: Sized;
}

impl Serialize for String {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        s.string(self)
    }
}

impl Serialize for str {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        s.string(self)
    }
}

impl Deserialize for String {
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError> {
        d.string()
    }
}

macro_rules! impl_serde {
    ($t:ident) => {
        impl Serialize for $t {
            fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
                s.$t(*self)
            }
        }

        impl Deserialize for $t {
            fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError> {
                d.$t()
            }
        }
    };
}

impl_serde!(bool);
impl_serde!(i8);
impl_serde!(i16);
impl_serde!(i32);
impl_serde!(i64);
impl_serde!(i128);
impl_serde!(u8);
impl_serde!(u16);
impl_serde!(u32);
impl_serde!(u64);
impl_serde!(u128);

impl<T: Serialize> Serialize for [T] {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        s.list(self)
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError> {
        d.vec()
    }
}

impl<T: Serialize> Serialize for &T {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        T::serialize(*self, s)
    }
}

impl<T: Serialize> Serialize for Rc<T> {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        let index = s.serialize_once(&**self)?;
        s.u32(index)
    }
}

impl<T: Deserialize> Deserialize for Rc<T> where Rc<T>: Any {
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError> {
        let index = d.u32()?;
        d.deserialize_once(index, |mut deserializer| {
            let val = deserializer.deserialize::<T>()?;
            Rc::try_new(val)
                // FIXME: Make allocation errors distinguishable from parsing errors.
                .map_err(|AllocError| DeserializeError)
        })?
    }
}

impl<T: Serialize> Serialize for Arc<T> {
    fn serialize<S: Serializer + ?Sized>(&self, s: &mut S) -> Result<(), SerializeError> {
        let index = s.serialize_once(&**self)?;
        s.u32(index)
    }
}

impl<T: Deserialize> Deserialize for Arc<T> where Arc<T>: Any {
    fn deserialize<D: Deserializer + ?Sized>(d: &mut D) -> Result<Self, DeserializeError> {
        let index = d.u32()?;
        d.deserialize_once(index, |mut deserializer| {
            let val = deserializer.deserialize::<T>()?;
            Arc::try_new(val)
                // FIXME: Make allocation errors distinguishable from parsing errors.
                .map_err(|AllocError| DeserializeError)
        })?
    }
}

/// An error type to be returned in the event of a failed serialization.
#[derive(Debug)]
pub struct SerializeError;

impl fmt::Display for SerializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "serialization error")
    }
}

impl error::Error for SerializeError {}

/// An error type to be returned in the event of a failed deserialization.
#[derive(Debug)]
pub struct DeserializeError;

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "deserialization error")
    }
}

impl error::Error for DeserializeError {}
