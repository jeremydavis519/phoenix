/* Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines things that are going to be standard in Rust but are not yet.

use {
    alloc::vec::Vec,
    core::fmt
};

// FIXME: Remove this as soon as Rust's standard `String` type allows a custom allocator.
#[derive(Debug, Clone)]
pub struct String<A: alloc::alloc::Allocator> {
    bytes: Vec<u8, A>
}

impl<A: alloc::alloc::Allocator> String<A> {
    pub fn new_in(allocator: A) -> Self {
        Self {
            bytes: Vec::new_in(allocator)
        }
    }

    pub fn with_capacity_in(capacity: usize, allocator: A) -> Self {
        Self {
            bytes: Vec::with_capacity_in(capacity, allocator)
        }
    }

    pub fn from_in(s: &str, allocator: A) -> Self {
        let bytes_slice = s.as_bytes();
        let mut bytes = Vec::with_capacity_in(bytes_slice.len(), allocator);
        bytes.extend_from_slice(bytes_slice);
        Self { bytes }
    }

    pub fn from_char_in(c: char, allocator: A) -> Self {
        let mut buffer = [0u8; 4];
        Self::from_in(c.encode_utf8(&mut buffer), allocator)
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..]) }
    }

    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { core::str::from_utf8_unchecked_mut(&mut self.bytes[..]) }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn push(&mut self, c: char) {
        let mut buffer = [0u8; 4];
        self.push_str(c.encode_utf8(&mut buffer));
    }

    pub fn push_str(&mut self, s: &str) {
        self.bytes.extend_from_slice(s.as_bytes());
    }

    pub fn pop(&mut self) -> Option<char> {
        if self.len() == 0 {
            return None;
        }
        let mut buffer = [0u8; 4];
        for i in (0 .. buffer.len()).rev() {
            buffer[i] = self.bytes.pop().unwrap();
            if buffer[i] & 0xc0 != 0x80 {
                return unsafe { core::str::from_utf8_unchecked(&buffer[i .. buffer.len()]).chars().next() };
            }
        }
        unreachable!()
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }
}

impl<A: alloc::alloc::Allocator> core::ops::Deref for String<A> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<A: alloc::alloc::Allocator> core::ops::DerefMut for String<A> {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<A: alloc::alloc::Allocator> PartialEq<String<A>> for String<A> {
    fn eq(&self, other: &String<A>) -> bool {
        self == other.as_str()
    }
}

impl<A: alloc::alloc::Allocator> PartialEq<&str> for String<A> {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl<A: alloc::alloc::Allocator> PartialEq<str> for String<A> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<A: alloc::alloc::Allocator> core::ops::AddAssign<&str> for String<A> {
    fn add_assign(&mut self, other: &str) {
        self.push_str(other);
    }
}

impl<A: alloc::alloc::Allocator> fmt::Display for String<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn to_ascii_lowercase<A: alloc::alloc::Allocator>(s: &str, allocator: A) -> String<A> {
    let mut s = String::from_in(s, allocator);
    s.make_ascii_lowercase();
    s
}
