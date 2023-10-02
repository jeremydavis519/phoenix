/* Copyright (c) 2023 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines the default serializer and deserializer for communication between different
//! processes and between processes and the kernel. The format is designed to be somewhat compact
//! and, more importantly, backward- and forward-compatible.

use {
    alloc::{
        vec,
        vec::Vec,
    },
    core::{
        convert::{TryInto, TryFrom},
        mem,
        num::NonZeroUsize,
        ops::Deref,
        str,
    },
    hashbrown::HashMap,
    super::{Serializer, Deserializer, Serialize, Deserialize, SerializeError, DeserializeError},
};

const VERSION: u16 = 0x0100;

pub(crate) struct DefaultSerializer {
    bytes: Vec<u8>,
    uniques_bytes: Vec<u8>,
    uniques: HashMap<*const (), u64>,
}

impl DefaultSerializer {
    pub(crate) fn new() -> Self {
        Self {
            bytes: Vec::new(),
            uniques_bytes: Vec::new(),
            // FIXME: HashMap::new() is vulnerable to HashDoS attacks. Use
            //        HashMap::with_hasher(ahash::RandomState::new()) instead.
            //        This will require a source of randomness, ideally a system call that returns
            //        cryptographic-quality random numbers.
            //        (https://docs.rs/hashbrown/0.14.0/hashbrown/struct.HashMap.html)
            uniques: HashMap::new(),
        }
    }

    pub(crate) fn finish(mut self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10 + self.bytes.len() + self.uniques_bytes.len());
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        let len = u64::try_from(self.bytes.len()).expect("serialized value's length is 2^64 or greater");
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.append(&mut self.bytes);
        bytes.append(&mut self.uniques_bytes);
        bytes
    }
}

impl Serializer for DefaultSerializer {
    type FieldSerializer = Self;

    fn str(&mut self, value: &str) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }

        let value = value.as_bytes();
        let Ok(len) = u64::try_from(value.len()) else { return Err(SerializeError) };

        self.bytes.reserve_exact(5 + value.len());
        self.bytes.push(Tag::String.into());
        self.bytes.extend_from_slice(&len.to_le_bytes());
        self.bytes.extend_from_slice(value);

        Ok(())
    }

    fn bool(&mut self, value: bool) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.push(if value { Tag::True.into() } else { Tag::False.into() });
        Ok(())
    }

    fn i8(&mut self, value: i8) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(2);
        self.bytes.push(Tag::I8.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn i16(&mut self, value: i16) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(3);
        self.bytes.push(Tag::I16.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn i32(&mut self, value: i32) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(5);
        self.bytes.push(Tag::I32.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn i64(&mut self, value: i64) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(9);
        self.bytes.push(Tag::I64.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn i128(&mut self, value: i128) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(17);
        self.bytes.push(Tag::I128.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(2);
        self.bytes.push(Tag::U8.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn u16(&mut self, value: u16) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(3);
        self.bytes.push(Tag::U16.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn u32(&mut self, value: u32) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(5);
        self.bytes.push(Tag::U32.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn u64(&mut self, value: u64) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(9);
        self.bytes.push(Tag::U64.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn u128(&mut self, value: u128) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }
        self.bytes.reserve_exact(17);
        self.bytes.push(Tag::U128.into());
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn list<T: Serialize, I: IntoIterator<Item = T>>(&mut self, values: I) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }

        // Tag followed by length
        let mut bytes = vec![Tag::List.into(), 0, 0, 0, 0, 0, 0, 0, 0];
        let mut len: u64 = 0;

        for value in values.into_iter() {
            value.serialize(self)?;
            bytes.append(&mut self.bytes);
            len += 1;
        }

        // Patch the length.
        bytes[1 .. 9].copy_from_slice(&len.to_le_bytes());

        self.bytes = bytes;
        Ok(())
    }

    fn object<S, I, F>(&mut self, field_names: I, serialize: F) -> Result<(), SerializeError>
        where S: Deref<Target = str>,
              I: IntoIterator<Item = S>,
              F: Fn(&mut Self::FieldSerializer, usize) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }

        let mut bytes = vec![Tag::Object.into(), 0, 0, 0, 0, 0, 0, 0, 0];
        let mut len: u64 = 0;

        for (i, name) in field_names.into_iter().enumerate() {
            // Field name
            self.str(&*name)?;
            bytes.append(&mut self.bytes);

            // Field value
            serialize(self, i)?;
            bytes.append(&mut self.bytes);

            len += 1;
        }

        // Patch the length.
        bytes[1 .. 9].copy_from_slice(&len.to_le_bytes());

        self.bytes = bytes;
        Ok(())
    }

    fn serialize_once<T: Serialize, P: Deref<Target = T>>(&mut self, value: P) -> Result<(), SerializeError> {
        if !self.bytes.is_empty() { return Err(SerializeError); }

        let value_ptr = &value as *const _ as *const ();
        let pointer = match self.uniques.get(&value_ptr) {
            Some(&pointer) => pointer,
            None => {
                value.serialize(self)?;
                let Ok(pointer) = u64::try_from(self.uniques_bytes.len()) else { return Err(SerializeError) };
                self.uniques_bytes.append(&mut self.bytes);
                self.uniques.insert(value_ptr, pointer);
                pointer
            },
        };
        self.u64(pointer)?;

        Ok(())
    }
}

pub(crate) struct DefaultDeserializer<'a> {
    bytes: &'a [u8],
    uniques_bytes: &'a [u8],
    uniques: HashMap<u64, *const ()>,
    value_length: Option<NonZeroUsize>,
}

impl<'a> DefaultDeserializer<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Option<Self> {
        if bytes.len() < 10 { return None; }

        let version = u16::from_le_bytes(bytes[0 .. 2].try_into().unwrap());
        if version > VERSION { return None; }

        let Ok(len) = usize::try_from(u64::from_le_bytes(bytes[2 .. 10].try_into().unwrap())) else { return None };
        if bytes.len() < 10 + len { return None; }
        let (bytes, uniques_bytes) = bytes[10 .. ].split_at(len);

        Some(Self { bytes, uniques_bytes, uniques: HashMap::new(), value_length: None })
    }
}

impl<'a> Deserializer for DefaultDeserializer<'a> {
    type FieldDeserializer = &'a mut Self;
    type OnceDeserializer = &'a mut Self;

    fn str(&mut self) -> Result<&str, DeserializeError> {
        if self.bytes[0] != Tag::String.into() { return Err(DeserializeError); }

        let string_len = usize::try_from(u64::from_le_bytes(self.bytes[1 .. 9].try_into().unwrap()))
            .expect("serialized string length doesn't fit in a usize");
        if string_len < self.bytes.len() - 9 { return Err(DeserializeError); }

        match str::from_utf8(&self.bytes[9 .. 9 + string_len]) {
            Ok(s) => {
                self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(9 + string_len) });
                Ok(s)
            },
            Err(_) => Err(DeserializeError),
        }
    }

    fn bool(&mut self) -> Result<bool, DeserializeError> {
        if self.bytes.len() < 1 { return Err(DeserializeError); }

        let Some(&tag) = self.bytes.get(0) else { return Err(DeserializeError) };
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(1) });

        if tag == Tag::True.into() { return Ok(true); }
        if tag == Tag::False.into() { return Ok(false); }

        self.value_length = None;
        Err(DeserializeError)
    }

    fn i8(&mut self) -> Result<i8, DeserializeError> {
        if self.bytes.len() < 2 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::I8.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(2) });
        Ok(i8::from_le_bytes(self.bytes[1 .. 2].try_into().unwrap()))
    }

    fn i16(&mut self) -> Result<i16, DeserializeError> {
        if self.bytes.len() < 3 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::I16.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(3) });
        Ok(i16::from_le_bytes(self.bytes[1 .. 3].try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, DeserializeError> {
        if self.bytes.len() < 5 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::I32.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(5) });
        Ok(i32::from_le_bytes(self.bytes[1 .. 5].try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, DeserializeError> {
        if self.bytes.len() < 9 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::I64.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(9) });
        Ok(i64::from_le_bytes(self.bytes[1 .. 9].try_into().unwrap()))
    }

    fn i128(&mut self) -> Result<i128, DeserializeError> {
        if self.bytes.len() < 17 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::I128.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(17) });
        Ok(i128::from_le_bytes(self.bytes[1 .. 17].try_into().unwrap()))
    }

    fn u8(&mut self) -> Result<u8, DeserializeError> {
        if self.bytes.len() < 2 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::U8.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(2) });
        Ok(u8::from_le_bytes(self.bytes[1 .. 2].try_into().unwrap()))
    }

    fn u16(&mut self) -> Result<u16, DeserializeError> {
        if self.bytes.len() < 3 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::U16.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(3) });
        Ok(u16::from_le_bytes(self.bytes[1 .. 3].try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, DeserializeError> {
        if self.bytes.len() < 5 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::U32.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(5) });
        Ok(u32::from_le_bytes(self.bytes[1 .. 5].try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, DeserializeError> {
        if self.bytes.len() < 9 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::U64.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(9) });
        Ok(u64::from_le_bytes(self.bytes[1 .. 9].try_into().unwrap()))
    }

    fn u128(&mut self) -> Result<u128, DeserializeError> {
        if self.bytes.len() < 17 { return Err(DeserializeError); }

        if self.bytes[0] != Tag::U128.into() { return Err(DeserializeError); }
        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(17) });
        Ok(u128::from_le_bytes(self.bytes[1 .. 17].try_into().unwrap()))
    }

    fn vec<T: Deserialize>(&mut self) -> Result<Vec<T>, DeserializeError> {
        if self.bytes.len() < 9 { return Err(DeserializeError); }
        if self.bytes[0] != Tag::List.into() { return Err(DeserializeError); }

        let Ok(len) = usize::try_from(u64::from_le_bytes(self.bytes[1 .. 9].try_into().unwrap()))
            else { return Err(DeserializeError) };
        let mut vec = Vec::with_capacity(len);
        let mut index = 9;
        for _ in 0 .. len {
            let mut deserializer = Self {
                bytes: &self.bytes[index .. ],
                uniques_bytes: self.uniques_bytes,
                uniques: HashMap::new(),
                value_length: None
            };
            vec.push(T::deserialize(&mut deserializer)?);
            index += deserializer.value_length.unwrap().get();
        }

        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(index) });
        Ok(vec)
    }

    fn object<F: FnMut(&str, Self::FieldDeserializer) -> Result<(), DeserializeError>>(&mut self, mut deserialize_field: F)
            -> Result<(), DeserializeError> {
        if self.bytes.len() < 9 { return Err(DeserializeError); }
        if self.bytes[0] != Tag::Object.into() { return Err(DeserializeError); }

        let Ok(len) = usize::try_from(u64::from_le_bytes(self.bytes[1 .. 9].try_into().unwrap()))
            else { return Err(DeserializeError) };
        let mut index = 9;
        for _ in 0 .. len {
            // Field name
            let mut name_deserializer = Self {
                bytes: &self.bytes[index .. ],
                uniques_bytes: self.uniques_bytes,
                uniques: HashMap::new(),
                value_length: None
            };
            let field_name = name_deserializer.str()?;

            // Value
            let mut value_deserializer = Self {
                bytes: &self.bytes[index .. ],
                uniques_bytes: self.uniques_bytes,
                uniques: HashMap::new(),
                value_length: None
            };
            deserialize_field(
                field_name,
                // Transmute to modify lifetimes.
                // SAFETY: The deserialized value is valid as long as `self.bytes` and
                // `self.uniques_bytes` don't change, and those fields will never change as long as
                // `*self` exists.
                unsafe { mem::transmute::<&mut Self, &'a mut Self>(&mut value_deserializer) },
            )?;

            index += name_deserializer.value_length.unwrap().get() + value_deserializer.value_length.unwrap().get();
        }

        self.value_length = Some(unsafe { NonZeroUsize::new_unchecked(index) });
        Ok(())
    }

    fn deserialize_once<T, P, F, G>(&mut self, deserialize: F, retrieve: G) -> Result<P, DeserializeError>
            where T: Deserialize,
                  P: Deref<Target = T>,
                  F: FnOnce(Self::OnceDeserializer) -> Result<P, DeserializeError>,
                  G: FnOnce(*const ()) -> P {
        let pointer = self.u64()?;
        match self.uniques.get(&pointer) {
            Some(&value) => Ok(retrieve(value)),
            None => {
                let Ok(index) = usize::try_from(pointer) else { return Err(DeserializeError) };
                let outer_bytes = mem::replace(&mut self.bytes, &self.uniques_bytes[index .. ]);
                let value_length = self.value_length;
                let value = deserialize(
                    // Transmute to modify lifetimes.
                    // SAFETY: The deserialized value is valid as long as `self.uniques_bytes`
                    // doesn't change, and that field will never change as long as `*self` exists.
                    unsafe { mem::transmute::<&mut Self, &'a mut Self>(self) }
                )?;
                self.bytes = outer_bytes;
                self.value_length = value_length;
                self.uniques.insert(pointer, &*value as *const T as *const ());
                Ok(value)
            },
        }
    }
}

macro_rules! u8_enum {
    ($vis:vis enum $type:ident {
        $($variant:ident = $val:expr),* $(,)?
    }) => {
        #[repr(u8)]
        $vis enum $type {
            $($variant = $val,)*
        }

        impl From<$type> for ::core::primitive::u8 {
            fn from(val: $type) -> ::core::primitive::u8 {
                val as ::core::primitive::u8
            }
        }

        impl TryFrom<::core::primitive::u8> for $type {
            type Error = TryIntoU8EnumError;
            fn try_from(val: ::core::primitive::u8) -> Result<$type, Self::Error> {
                match val {
                    $($val => ::core::result::Result::Ok(Self::$variant),)*
                    _ => ::core::result::Result::Err(TryIntoU8EnumError),
                }
            }
        }
    };
}

u8_enum! {
    enum Tag {
        I8     = 1,
        I16    = 2,
        I32    = 3,
        I64    = 4,
        I128   = 5,
        U8     = 6,
        U16    = 7,
        U32    = 8,
        U64    = 9,
        U128   = 10,
        True   = b'T',
        False  = b'F',
        String = b'"',
        List   = b'[',
        Object = b'{',
    }
}

struct TryIntoU8EnumError;
