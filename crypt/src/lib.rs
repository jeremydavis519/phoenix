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

//! This crate defines all the built-in cryptographic capabilities of Phoenix.

//#![no_std]

#![feature(inline_const)]
#![feature(iter_array_chunks)]
#![feature(maybe_uninit_uninit_array_transpose)]
#![feature(wrapping_int_impl)]

use core::{
    convert::{TryFrom, TryInto},
    iter,
    mem::{self, MaybeUninit},
    num::Wrapping,
};

pub fn sha512(bytes: &[u8]) -> [u8; 64] {
    // SHA-512 round constants
    static K: [Wrapping<u64>; 80] = [
        Wrapping(0x428a2f98d728ae22), Wrapping(0x7137449123ef65cd), Wrapping(0xb5c0fbcfec4d3b2f), Wrapping(0xe9b5dba58189dbbc),
        Wrapping(0x3956c25bf348b538), Wrapping(0x59f111f1b605d019), Wrapping(0x923f82a4af194f9b), Wrapping(0xab1c5ed5da6d8118),
        Wrapping(0xd807aa98a3030242), Wrapping(0x12835b0145706fbe), Wrapping(0x243185be4ee4b28c), Wrapping(0x550c7dc3d5ffb4e2),
        Wrapping(0x72be5d74f27b896f), Wrapping(0x80deb1fe3b1696b1), Wrapping(0x9bdc06a725c71235), Wrapping(0xc19bf174cf692694),
        Wrapping(0xe49b69c19ef14ad2), Wrapping(0xefbe4786384f25e3), Wrapping(0x0fc19dc68b8cd5b5), Wrapping(0x240ca1cc77ac9c65), 
        Wrapping(0x2de92c6f592b0275), Wrapping(0x4a7484aa6ea6e483), Wrapping(0x5cb0a9dcbd41fbd4), Wrapping(0x76f988da831153b5),
        Wrapping(0x983e5152ee66dfab), Wrapping(0xa831c66d2db43210), Wrapping(0xb00327c898fb213f), Wrapping(0xbf597fc7beef0ee4),
        Wrapping(0xc6e00bf33da88fc2), Wrapping(0xd5a79147930aa725), Wrapping(0x06ca6351e003826f), Wrapping(0x142929670a0e6e70),
        Wrapping(0x27b70a8546d22ffc), Wrapping(0x2e1b21385c26c926), Wrapping(0x4d2c6dfc5ac42aed), Wrapping(0x53380d139d95b3df),
        Wrapping(0x650a73548baf63de), Wrapping(0x766a0abb3c77b2a8), Wrapping(0x81c2c92e47edaee6), Wrapping(0x92722c851482353b), 
        Wrapping(0xa2bfe8a14cf10364), Wrapping(0xa81a664bbc423001), Wrapping(0xc24b8b70d0f89791), Wrapping(0xc76c51a30654be30),
        Wrapping(0xd192e819d6ef5218), Wrapping(0xd69906245565a910), Wrapping(0xf40e35855771202a), Wrapping(0x106aa07032bbd1b8),
        Wrapping(0x19a4c116b8d2d0c8), Wrapping(0x1e376c085141ab53), Wrapping(0x2748774cdf8eeb99), Wrapping(0x34b0bcb5e19b48a8),
        Wrapping(0x391c0cb3c5c95a63), Wrapping(0x4ed8aa4ae3418acb), Wrapping(0x5b9cca4f7763e373), Wrapping(0x682e6ff3d6b2b8a3),
        Wrapping(0x748f82ee5defb2fc), Wrapping(0x78a5636f43172f60), Wrapping(0x84c87814a1f0ab72), Wrapping(0x8cc702081a6439ec), 
        Wrapping(0x90befffa23631e28), Wrapping(0xa4506cebde82bde9), Wrapping(0xbef9a3f7b2c67915), Wrapping(0xc67178f2e372532b),
        Wrapping(0xca273eceea26619c), Wrapping(0xd186b8c721c0c207), Wrapping(0xeada7dd6cde0eb1e), Wrapping(0xf57d4f7fee6ed178),
        Wrapping(0x06f067aa72176fba), Wrapping(0x0a637dc5a2c898a6), Wrapping(0x113f9804bef90dae), Wrapping(0x1b710b35131c471b),
        Wrapping(0x28db77f523047d84), Wrapping(0x32caab7b40c72493), Wrapping(0x3c9ebe0a15c9bebc), Wrapping(0x431d67c49c100d4c),
        Wrapping(0x4cc5d4becb3e42b6), Wrapping(0x597f299cfc657e2a), Wrapping(0x5fcb6fab3ad6faec), Wrapping(0x6c44198c4a475817),
    ];

    // Initial hash values
    let mut hash: [Wrapping<u64>; 8] = [
        Wrapping(0x6a09e667f3bcc908),
        Wrapping(0xbb67ae8584caa73b),
        Wrapping(0x3c6ef372fe94f82b),
        Wrapping(0xa54ff53a5f1d36f1),
        Wrapping(0x510e527fade682d1),
        Wrapping(0x9b05688c2b3e6c1f),
        Wrapping(0x1f83d9abfb41bd6b),
        Wrapping(0x5be0cd19137e2179),
    ];

    let len_bytes = (u128::try_from(bytes.len()).unwrap() * 8).to_be_bytes();

    let chunks = bytes.iter()
        // Padding: a single 1 followed by enough bits to finish a 1024-bit chunk with a 128-bit
        // length at the end
        .chain(iter::once(&0x80))
        .chain(iter::repeat(&0x00).take((256 - bytes.len() % 128 - mem::size_of::<u8>() - mem::size_of::<u128>()) % 128))
        .chain(len_bytes.iter())
        // Simplify later use
        .map(|&x| x)
        // Split into 1024-bit chunks
        .array_chunks::<128>();

    for chunk in chunks {
        // Initialize message schedule array from chunk
        let mut w = [const { MaybeUninit::uninit() }; 80];
        for i in 0 .. chunk.len() / mem::size_of::<u64>() {
            w[i].write(Wrapping(u64::from_be_bytes(chunk[i * mem::size_of::<u64>() .. (i + 1) * mem::size_of::<u64>()].try_into().unwrap())));
        }
        for i in chunk.len() / mem::size_of::<u64>() .. w.len() {
            let wi = |n| unsafe { MaybeUninit::<Wrapping<u64>>::assume_init_read(&w[i - n]) };
            let s0 = wi(15).rotate_right(1) ^ wi(15).rotate_right(8) ^ wi(15) >> 7;
            let s1 = wi(2).rotate_right(19) ^ wi(2).rotate_right(61) ^ wi(2) >> 6;
            w[i].write(wi(16) + s0 + wi(7) + s1);
        }
        let w = unsafe { w.transpose().assume_init() };

        // Working variables initialized to current hash value
        let mut v = hash;

        // Compression function main loop
        for i in 0 .. w.len() {
            let s1 = v[4].rotate_right(14) ^ v[4].rotate_right(18) ^ v[4].rotate_right(41);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let temp1 = v[7] + s1 + ch + K[i] + w[i];
            let s0 = v[0].rotate_right(28) ^ v[0].rotate_right(34) ^ v[0].rotate_right(39);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let temp2 = s0 + maj;

            v[7] = v[6];
            v[6] = v[5];
            v[5] = v[4];
            v[4] = v[3] + temp1;
            v[3] = v[2];
            v[2] = v[1];
            v[1] = v[0];
            v[0] = temp1 + temp2;
        }

        // Add the compressed chunk to the hash value.
        for i in 0 .. hash.len() {
            hash[i] += v[i];
        }
    }

    let mut hash_bytes = [0; 64];
    for i in 0 .. 8 {
        hash_bytes[i * 8 .. (i + 1) * 8].copy_from_slice(&hash[i].0.to_be_bytes());
    }
    hash_bytes
}
