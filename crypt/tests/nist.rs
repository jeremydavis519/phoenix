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

//! These tests are based on the test vectors provided by the NIST for informally verifying the
//! operation of SHA functions.

#![feature(generators, generator_trait)]
#![feature(inline_const)]
#![feature(iter_array_chunks)]

use {
    std::{
        fs::File,
        io::{BufReader, BufRead},
        path::PathBuf,
    },
    crypt::sha512,
};

#[test]
fn sha512_short_msg() {
    sha512_test("SHA512ShortMsg.rsp");
}

#[test]
fn sha512_long_msg() {
    sha512_test("SHA512LongMsg.rsp");
}

#[test]
fn sha512_monte_carlo() {
    sha512_test("SHA512Monte.txt");
}

fn sha512_test(rsp_filename: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("shabytetestvectors");
    path.push(rsp_filename);
    let rsp = File::open(path)
        .expect("failed to open RSP file");
    let rsp = BufReader::new(rsp);

    let mut len = None;
    let mut msg = None;

    // Monte Carlo test state
    let mut m = Vec::new();
    let mut md = [const { [0; 64] }; 1003];
    let mut i = 3;
    let mut j = 0;
    let mut checkpoint_i = None;
    let mut checkpoint_j = None;

    for line in rsp.lines() {
        // This is a very rough approximation of a proper parser, enough to run the tests but
        // nothing more.
        let line = line.expect("failed to read a line");
        let line = line.trim();
        if line.is_empty() || line.starts_with("#") || line.starts_with("[") {
            continue;
        }

        let (var, value) = line.split_once("=")
            .expect("missing `=` in `{line}`");

        let var = var.trim();
        let value = value.trim();

        if var == "Len" {
            let bitlen = usize::from_str_radix(value, 10)
                .expect("invalid length");
            assert_eq!(bitlen % 8, 0);
            len = Some(bitlen / 8);
            continue;
        }

        if var == "Msg" {
            msg = Some(
                value.chars().array_chunks::<2>()
                    .map(|cs| u8::from_str_radix(&cs.iter().collect::<String>(), 16).expect("invalid message"))
                    .collect::<Vec<u8>>()
            );
            continue;
        }

        if var == "Seed" {
            let seed = value.chars().array_chunks::<2>()
                .map(|cs| u8::from_str_radix(&cs.iter().collect::<String>(), 16).expect("invalid seed"))
                .collect::<Vec<u8>>();
            md[0].copy_from_slice(&seed);
            md[1].copy_from_slice(&seed);
            md[2].copy_from_slice(&seed);
            i = 3;
            j = 0;
            m = md[0].iter()
                .chain(md[1].iter())
                .chain(md[2].iter())
                .map(|&x| x)
                .collect();
            continue;
        }

        if var == "COUNT" {
            checkpoint_j = Some(
                usize::from_str_radix(value, 10)
                    .expect("invalid count")
            );
            continue;
        }

        if var == "i" {
            checkpoint_i = Some(
                usize::from_str_radix(value, 10)
                    .expect("invalid i")
            );
            continue;
        }

        if var == "M" {
            do_monte_carlo(&mut i, &mut j, checkpoint_i, checkpoint_j, &mut md, &mut m);
            let expected = value.chars().array_chunks::<2>()
                .map(|cs| u8::from_str_radix(&cs.iter().collect::<String>(), 16).expect("invalid message"))
                .collect::<Vec<u8>>();
            assert_eq!(m, expected);
            continue;
        }

        if var == "MDi" {
            let mdi = do_monte_carlo(&mut i, &mut j, checkpoint_i, checkpoint_j, &mut md, &mut m);
            let expected = value.chars().array_chunks::<2>()
                .map(|cs| u8::from_str_radix(&cs.iter().collect::<String>(), 16).expect("invalid message digest"))
                .collect::<Vec<u8>>();
            assert_eq!(&mdi[ .. ], &expected[ .. ]);
            continue;
        }

        if var == "MD" {
            let md = if checkpoint_j.is_some() {
                do_monte_carlo(&mut i, &mut j, None, checkpoint_j, &mut md, &mut m)
            } else {
                let msg = msg.as_ref().expect("missing message");
                let len = len.expect("missing length");
                sha512(&msg[0 .. len])
            };
            let expected = value.chars().array_chunks::<2>()
                .map(|cs| u8::from_str_radix(&cs.iter().collect::<String>(), 16).expect("invalid message digest"))
                .collect::<Vec<u8>>();
            assert_eq!(&md, &expected[ .. ]);
            continue;
        }

        panic!("unrecognized variable `{var}`");
    }
}

fn do_monte_carlo(
    i: &mut usize,
    j: &mut usize,
    checkpoint_i: Option<usize>,
    checkpoint_j: Option<usize>,
    md: &mut [[u8; 64]; 1003],
    m: &mut Vec<u8>,
) -> [u8; 64] {
    let checkpoint_j = checkpoint_j.expect("no checkpoint");
    let checkpoint_i = checkpoint_i.unwrap_or(1002);

    loop {
        *m = md[*i - 3].iter()
            .chain(md[*i - 2].iter())
            .chain(md[*i - 1].iter())
            .map(|&x| x)
            .collect();
        md[*i] = sha512(m);

        if *j == checkpoint_j && *i == checkpoint_i {
            break;
        }

        *i += 1;
        if *i == 1003 {
            md[0] = md[1002];
            md[1] = md[1002];
            md[2] = md[1002];
            *j += 1;
            *i = 3;
        }
    }

    return md[*i];
}
