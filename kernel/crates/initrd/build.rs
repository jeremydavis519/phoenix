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

//! This build script generates the InitRD contents for inclusion in the kernel whenever a file is
//! added to or removed from one of the relevant "arch/*/initrd" directories and whenever the
//! target architecture/machine changes.

use std::{
    collections::HashMap,
    env,
    fs,
    io::{self, BufWriter, Error, ErrorKind, Read, Write},
    path::PathBuf
};

static CONTENTS_PATH: &str = "contents";

fn main() {
    println!("cargo:rerun-if-env-changed=PHOENIX_TARGET");
    let target = env::var("PHOENIX_TARGET")
        .expect("unknown target machine (PHOENIX_TARGET is not set)");

    // Build the InitRD.
    let output_path = PathBuf::from(env::var("OUT_DIR")
        .expect("unknown output directory (OUT_DIR is not set)"))
        .join("contents.rs");
    let file = match fs::File::create(output_path.clone()) {
        Ok(file) => file,
        Err(err) => panic!("could not create file {}: {}", output_path.to_str().expect("non-UTF-8 path"), err)
    };
    let root = match make_initrd(&target) {
        Ok(root) => root,
        Err(err) => panic!("could not construct InitRD: {}", err)
    };
    match write_initrd(file, root) {
        Ok(()) => {},
        Err(err) => panic!("could not write {}: {}", output_path.to_str().expect("non-UTF-8 path"), err)
    };
}

fn make_initrd(target: &str) -> io::Result<InitRDDirBuilder> {
    let mut path = PathBuf::from(CONTENTS_PATH);

    let subpath = path.join("initrd");
    let subpath_str = subpath.to_str().expect("non-UTF-8 path");
    println!("cargo:rerun-if-changed={}", subpath_str);
    let mut root = if let Ok(initrd_dir) = subpath.read_dir() {
        // Handle the most general layer before narrowing down on a target.
        make_initrd_piece(initrd_dir)?
    } else {
        // This directory doesn't exist yet. Make it so Cargo can keep track of its changes.
        // (Otherwise, it will assume the directory was deleted and will run this build script
        // again.)
        match fs::DirBuilder::new()
                .recursive(true)
                .create(subpath.clone()) {
            Ok(()) => {},
            Err(e) => println!("cargo:warning=failed to create directory `{}`: {}", subpath_str, e)
        };
        InitRDDirBuilder::new()
    };
    for target_piece in target.split('/') {
        path.push(target_piece);
        // Add layers as they are found.
        let subpath = path.join("initrd");
        let subpath_str = subpath.to_str().expect("non-UTF-8 path");
        println!("cargo:rerun-if-changed={}", subpath_str);
        if let Ok(initrd_dir) = subpath.read_dir() {
            root = merge_initrd_pieces(root, make_initrd_piece(initrd_dir)?);
        } else {
            // This directory doesn't exist yet. Make it so Cargo can keep track of its changes.
            // (Otherwise, it will assume the directory was deleted and will run this build script
            // again.)
            match fs::DirBuilder::new()
                    .recursive(true)
                    .create(subpath.clone()) {
                Ok(()) => {},
                Err(e) => println!("cargo:warning=failed to create directory `{}`: {}", subpath_str, e)
            };
        }
    }

    Ok(root)
}

fn make_initrd_piece(dir: fs::ReadDir) -> io::Result<InitRDDirBuilder> {
    let mut root = InitRDDirBuilder::new();

    for entry in dir {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let mut subdir = make_initrd_piece(entry.path().read_dir()?)?;
            subdir.name = entry.file_name().into_string()
                .map_err(|name| Error::new(ErrorKind::InvalidData,
                    format!("filename `{}` cannot be converted to UTF-8", name.to_string_lossy())))?;
            root.subdirs.insert(subdir.name.clone(), subdir);
        } else {
            let file_name = entry.file_name().into_string()
                .map_err(|name| Error::new(ErrorKind::InvalidData,
                    format!("filename `{}` cannot be converted to UTF-8", name.to_string_lossy())))?;
            root.files.insert(file_name, entry);
        }
    }

    Ok(root)
}

fn merge_initrd_pieces(general: InitRDDirBuilder, mut specific: InitRDDirBuilder) -> InitRDDirBuilder {
    let mut merged = general;

    for (name, subdir_spec) in specific.subdirs.drain() {
        if let Some(subdir_gen) = merged.subdirs.remove(&name) {
            // Name conflict found. Merge the subdirectories.
            let subdir_merged = merge_initrd_pieces(subdir_gen, subdir_spec);
            merged.subdirs.insert(name, subdir_merged);
        } else {
            // No name conflict. Add the new subdirectory.
            merged.subdirs.insert(name, subdir_spec);
        }
    }
    for (name, file_spec) in specific.files.drain() {
        // We don't care whether there's a name conflict. Always keep the more specific file.
        merged.files.insert(name, file_spec);
    }

    merged
}

fn write_initrd(file: fs::File, root: InitRDDirBuilder) -> io::Result<()> {
    let mut writer = BufWriter::new(file);

    write!(writer,
r#"// This file was automatically generated and should *not* be modified by hand. Instead, modify the
// files in the initrd crate's "contents/*/initrd" directories.

/// The root directory of the initial RAM disk. See the module documentation for details on how to
/// use it.
pub static ROOT: Directory = {};
"#, root.build_directory_code()?)
}

struct InitRDDirBuilder {
    name: String,
    subdirs: HashMap<String, InitRDDirBuilder>,
    files: HashMap<String, fs::DirEntry>
}

impl InitRDDirBuilder {
    pub fn new() -> Self {
        Self { name: String::from(""), subdirs: HashMap::new(), files: HashMap::new() }
    }

    /// Finishes building the directory by reading all the files it should contain and returns the
    /// Rust code that will represent it.
    fn build_directory_code(&self) -> io::Result<String> {
        Ok(format!("Directory{{name:\"{}\",subdirs:&[{}],files:&[{}]}}",
            self.name.chars().flat_map(|c| c.escape_default()).collect::<String>(),
            self.subdirs.values().map(|d| d.build_directory_code()).collect::<io::Result<String>>()?,
            self.files.iter().map(|(name, dir_entry)| {
                let mut file = fs::File::open(dir_entry.path())?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                Ok(format!("File{{name:\"{}\",contents:&[{}]}},",
                    name,
                    contents.iter().map(|b| format!("{},", b)).collect::<String>()
                ))
            }).collect::<io::Result<String>>()?
        ))
    }
}
