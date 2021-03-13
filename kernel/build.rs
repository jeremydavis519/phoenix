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

//! This is a build script. It does some setup before the crate is compiled, such as compiling
//! non-Rust code and telling Cargo to link it.

use std::path::PathBuf;

fn main() {
    if cfg!(feature = "unit-test") {
        // FIXME: This branch should be unnecessary when we start supporting the host architecture
        // (i.e. x86_64).
    } else {
        // Compile the relevant architecture-specific source files.
        let arch_dir = "src/arch";
        println!("cargo:rerun-if-changed={}", arch_dir);
        let o_files = match build_util::run_assembler(arch_dir) {
            Ok(files) => files,
            Err(e) => panic!("assembler failed: {}", e)
        };

        // Link the architecture-specific files into a static library.
        let mut lib_dir = PathBuf::from("lib");
        lib_dir.push(env!("PHOENIX_TARGET"));
        let pkg_name = env!("CARGO_PKG_NAME");
        match build_util::archive(o_files, lib_dir.join(format!("lib{}.a", pkg_name))) {
            Ok(()) => {},
            Err(e) => panic!("linker failed: {}", e)
        };

        // Tell Cargo to link to the newly created library.
        println!("cargo:rustc-link-lib=static={}", pkg_name);
        println!("cargo:rustc-link-search=native={}", lib_dir.to_str().expect("non-UTF-8 path"));
    }
}
