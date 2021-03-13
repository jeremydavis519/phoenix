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

//! This crate defines helper functions for all the build scripts in the Phoenix kernel.

use std::{
    env::{self, VarError},
    fmt,
    fs::{DirBuilder, ReadDir},
    path::{Path, PathBuf},
    io,
    process::{Command, ExitStatus}
};

/// Runs the assembler on all the assembly files in the given directory and its subdirectories,
/// returning an iterator over the paths to the resulting `.o` files. Note that this is done
/// lazily: if the build script never uses a given object file, that file may never be assembled.
pub fn run_assembler<P: AsRef<Path>>(source_dir: P) -> Result<impl Iterator<Item = Result<PathBuf, BuildError>>, BuildError> {
    let source_dir = PathBuf::from(source_dir.as_ref());

    let out_dir = PathBuf::from(env::var("OUT_DIR")
        .map_err(|e| BuildError::VarError("OUT_DIR", e))?);

    let assembler = env::var("PHOENIX_ASSEMBLER")
        .map_err(|e| BuildError::VarError("PHOENIX_ASSEMBLER", e))?;

    let cpu = env::var("PHOENIX_CPU")
        .map_err(|e| BuildError::VarError("PHOENIX_CPU", e))?;
    let symbols: &[&str] = match cpu.as_str() {
        "cortex-a35" | "cortex-a53" | "cortex-a57" | "cortex-a72" | "cortex-a73" =>
            &["-defsym", "_armv8_=1"],
        "cortex-a55" | "cortex-a75" | "cortex-a76" | "cortex-m1" | "thunderx" | "xgene1" | "xgene2" =>
            &["-defsym", "_armv8_=1", "-defsym", "_armv8_1_=1", "-defsym", "_armv8_2_=1"],
        "falkor" | "qdf24xx" | "saphira" | "vulcan" =>
            unimplemented!(), // TODO: Find out which CPU each of these is.
        x => panic!("unrecognized PHOENIX_CPU value `{}`", x)
    };

    let target = PathBuf::from(env::var("PHOENIX_TARGET")
        .map_err(|e| BuildError::VarError("PHOENIX_TARGET", e))?);

    let debug: &[&str] = if cfg!(debug_assertions) {
        &["-g"]
    } else {
        &[]
    };

    files_filter_map(
        source_dir.clone(),
        move |path| is_in_target(path.strip_prefix(&source_dir).unwrap(), &target) && is_assembly_file(path),
        move |path| {
            let out_path = out_dir.join(path).with_extension(".o");
            DirBuilder::new()
                .recursive(true)
                .create(out_path.parent().unwrap())
                .map_err(|e| BuildError::IoError(e))?;
            match Command::new(&assembler)
                    .arg(format!("-I{}", path.parent().unwrap().to_str().unwrap()))
                    .arg(format!("-mcpu={}", cpu))
                    .args(symbols)
                    .args(debug)
                    .arg(path)
                    .arg("-o").arg(&out_path)
                    .status() {
                Ok(status) if status.success() => Ok(out_path),
                Ok(status)                     => Err(BuildError::CompileError(status)),
                Err(e)                         => Err(BuildError::IoError(e))
            }
        }
    )
}

/// Archives all the object files returned by the given iterator, combining them into a single
/// statically linked library at the given path (which should be inside `OUT_DIR`).
pub fn archive<I, P, Q>(in_paths: I, out_path: P) -> Result<(), BuildError>
        where I: IntoIterator<Item = Result<Q, BuildError>>,
              P: AsRef<Path>,
              Q: AsRef<Path>
{
    let out_path = out_path.as_ref();

    let archiver = env::var("PHOENIX_ARCHIVER")
        .map_err(|e| BuildError::VarError("PHOENIX_ARCHIVER", e))?;

    // Make sure the directory we'll use exists.
    DirBuilder::new()
        .recursive(true)
        .create(out_path.parent().unwrap())
        .map_err(|e| BuildError::IoError(e))?;

    // FIXME: We need an equivalent of `make clean`. `ar r` only appends and replaces files in the
    // archive; it never deletes them. And since symbols are resolved according to the order in the
    // archive, it's possible for a symbol defined in a deleted source file to shadow one defined
    // in a new source file.

    let cmd = &mut Command::new(archiver);
    cmd.arg("rcTu")
        .arg(out_path);
    for path in in_paths {
        cmd.arg(path?.as_ref());
    }
    match cmd.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status)                     => Err(BuildError::LinkError(status)),
        Err(e)                         => Err(BuildError::IoError(e))
    }
}

// Determines whether the given file applies to the given target.
fn is_in_target<P: AsRef<Path>, Q: AsRef<Path>>(path: P, target: Q) -> bool {
    // Using this code, "aarch64" applies to "aarch64/qemu-virt", but not vice versa.
    target.as_ref().starts_with(path.as_ref().parent().unwrap())
}

/// Determines whether the given file is an assembly file, based on its extension.
pub fn is_assembly_file<P: AsRef<Path>>(path: P) -> bool {
    static ASSEMBLY_EXTENSIONS: &[&str] = &["asm", "s"];

    match path.as_ref().extension() {
        Some(ext) => ASSEMBLY_EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str()),
        None => false
    }
}

/// Lazily performs the given operation on every file in the given directory and its subdirectories
/// that matches the given filter. Returns an iterator over the results.
pub fn files_filter_map<P, F, M, T>(dir: P, filter: F, map: M) -> Result<impl Iterator<Item = Result<T, BuildError>>, BuildError>
        where P: AsRef<Path>,
              F: Fn(&Path) -> bool,
              M: Fn(&Path) -> Result<T, BuildError> {
    struct FilesIterator<T, F, M>
            where F: Fn(&Path) -> bool,
                  M: Fn(&Path) -> Result<T, BuildError> {
        directories: Vec<ReadDir>,
        filter: F,
        map: M
    }
    impl<T, F, M> Iterator for FilesIterator<T, F, M>
            where F: Fn(&Path) -> bool,
                  M: Fn(&Path) -> Result<T, BuildError> {
        type Item = Result<T, BuildError>;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                match self.directories.last_mut() {
                    None => return None,
                    Some(directory) => match directory.next() {
                        None => { // End of subdirectory
                            self.directories.pop();
                        },
                        Some(Err(e)) => return Some(Err(BuildError::IoError(e))),
                        Some(Ok(entry)) => match entry.file_type() {
                            Err(e) => return Some(Err(BuildError::IoError(e))),
                            Ok(file_type) if file_type.is_dir() => match entry.path().read_dir() { // Found subdirectory
                                Err(e) => return Some(Err(BuildError::IoError(e))),
                                Ok(directory) => {
                                    self.directories.push(directory);
                                }
                            },
                            Ok(_) => { // Found file
                                let path = entry.path();
                                if (self.filter)(&path) {
                                    return Some((self.map)(&path));
                                }
                            }
                        }
                    }
                };
            }
        }
    }

    Ok(FilesIterator {
        directories: vec![dir.as_ref().read_dir().map_err(|e| BuildError::IoError(e))?],
        filter,
        map
    })
}

#[derive(Debug)]
pub enum BuildError {
    CompileError(ExitStatus),
    LinkError(ExitStatus),
    IoError(io::Error),
    VarError(&'static str, VarError)
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::CompileError(ref status) => write!(f, "compiler exited with status `{}`", status),
            Self::LinkError(ref status)    => write!(f, "linker exited with status `{}`", status),
            Self::IoError(ref e)  => write!(f, "{}", e),
            Self::VarError(var_name, ref e) => write!(f, "{}: {}", e, var_name)
        }
    }
}
