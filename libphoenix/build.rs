/* Copyright (c) 2023-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

use std::{
    env,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    mem,
    path::PathBuf,
};

fn main() {
    let out_root = PathBuf::from(env::var("OUT_DIR")
        .expect("unknown output directory (OUT_DIR is not set)"))
        .join("posix");
    match fs::create_dir_all(&out_root) {
        Ok(()) => {},
        Err(e) => panic!("could not create directory {}: {}", out_root.display(), e),
    };

    let include_path = PathBuf::from("..")
        .join("libc")
        .join("include");

    // errno.h
    let in_path = include_path.join("errno.h");
    let out_path = out_root.join("errno.rs");
    println!("cargo:rerun-if-changed={}", in_path.display());

    let in_file = BufReader::new(File::open(in_path).expect("failed to open errno.h"));
    let mut out_file = BufWriter::new(File::create(out_path).expect("failed to create errno.rs"));

    writeln!(out_file, "// Automatically generated by a build script. Do not modify.").unwrap();
    writeln!(out_file).unwrap();
    writeln!(out_file, "/// Represents an error number defined in libc's errno.h").unwrap();
    writeln!(out_file, "#[allow(missing_docs)]").unwrap();
    writeln!(out_file, "#[repr(usize)]").unwrap();
    writeln!(out_file, "pub enum Errno {{").unwrap();

    let bytes = in_file.bytes();
    let mut line = Vec::new();
    for b in bytes {
        let b = b.expect("error reading errno.h");
        if b == b'\n' {
            let line_str = String::from_utf8(mem::replace(&mut line, Vec::new())).expect("errno.h isn't pure UTF-8");
            line.clear();

            let mut words = line_str.split_whitespace();
            let (Some(word0), Some(word1), Some(word2)) = (words.next(), words.next(), words.next()) else { continue };
            if words.next().is_some() { continue; }
            if word0 == "#define" && word2.chars().all(|c| c.is_ascii_digit()) {
                writeln!(out_file, "{} = {},", word1, word2).unwrap();
            }
        } else {
            line.push(b);
        }
    }

    writeln!(out_file, "}}").unwrap();

    out_file.flush().expect("failed to flush errno.rs");
}
