/* Copyright (c) 2019-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines the interface to Phoenix's filesystem and is analogous to the `std::fs`
//! module.
//!
//! In this filesystem, an absolute path is written like this:
//! ```text
//! :root/path/to/file
//! ```
//! `:root` is a placeholder for the name of the particular root (see the documentation for `Root`
//! in this module).
//!
//! As in most filesystems, relative paths are also allowed. To form one, simply omit the root and
//! the following slash. Omitting the root but including the slash does the same thing as in
//! Windows: it forms an absolute path with the same root as the working directory.

use {
    alloc::string::String,
    core::fmt,
    shared::ffi::CStrRef,
    io::{Read, Seek, SeekFrom}
    crate::error::Error
};

/// Represents a path in the file system.
#[derive(Debug, Clone, Copy)]
pub struct Path<'a> {
    raw: &'a str
}

impl<'a> Path<'a> {
    /// Makes a new `Path` from the given string.
    pub fn new(path: &'a str) -> Path<'a> {
        Path { raw: path }
    }

    /// Determines whether the path is absolute or relative.
    pub fn absolute(self) -> bool {
        self.raw.starts_with(':')
    }

    /// Splits this path into a file system root and a path relative to that root.
    /// If the path doesn't specify a root, the root is `Root::Relative`.
    pub fn split(self) -> (Root, Path<'a>) {
        if self.absolute() {
            let sep_index = self.raw.find('/');
            let root_len = sep_index.unwrap_or(self.raw.len());
            let stem_start = sep_index.unwrap_or(self.raw.len());

            let root = &self.raw[ .. root_len];
            let stem = &self.raw[stem_start .. ];

            // TODO: Is there a way to automate the exhaustive matching of roots (apart from
            // `Root::Relative`)? Maybe hash the strings and use `ffi_enum!` to convert from `u64`
            // to an enum variant?
            let root = match root {
                ":host" =>   Root::Host,
                ":initrd" => Root::Initrd,
                _ =>         Root::Unknown
            };

            (root, Path { raw: stem })
        } else {
            (Root::Relative, self)
        }
    }

    /// Converts the `Path` into a UTF-8 string.
    pub fn as_str(&self) -> &'a str {
        self.raw
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl<'a> From<Path<'a>> for String {
    fn from(path: Path) -> String {
        String::from(path.raw)
    }
}

impl<'a> From<&'a str> for Path<'a> {
    fn from(s: &str) -> Path {
        Path::new(s)
    }
}

impl<'a> From<Path<'a>> for &'a str {
    fn from(path: Path) -> &str {
        path.raw
    }
}

/// Represents the root of a file system and contains everything needed to uniquely identify it.
pub enum Root {
    /// `:host`: The host's filesystem when we are running as a guest (e.g. in a virtual machine like Qemu).
    Host,
    /// `:initrd`: The initial RAM disk.
    Initrd,
    /// (no root in the path): The current working directory.
    Relative,
    /// (any root other than those listed here): An unknown root, probably misspelled or nonexistent.
    Unknown
}

/// Represents a file somewhere in the filesystem.
#[derive(Debug)]
pub struct File {
    handle: FileHandle
}

// Represents the location of a file and a cursor inside it, whether that be on a disk or in RAM
// (as part of the initrd).
#[derive(Debug)]
enum FileHandle {
    Host(HostHandle),
    Ram(RamHandle),
    // TODO: Disk(some kind of handle)
}

#[derive(Debug)]
struct HostHandle {
    file: hosted::io::File
}

#[derive(Debug)]
struct RamHandle {
    file: &'static initrd::File<'static>,
    cursor: usize
}

impl File {
    /// Attempts to open a file in read-only mode.
    pub fn open<'a, P: Into<Path<'a>>>(path: P) -> io::Result<File> {
        let path = path.into();
        let (root, stem) = path.split();

        match root {
            Root::Host => {
                let mut stem = String::from(stem.as_str());
                if stem.starts_with('/') {
                    stem.remove(0);
                }
                match path_to_cstr(&mut stem) {
                    None => Err(io::ErrorKind::NotFound.into()),
                    Some(filename) => {
                        match hosted::io::File::open(filename, hosted::io::FileMode::ReadBin) {
                            Err(errno) => Err(io::Error::new(io::ErrorKind::Other, FsError::new(format!("errno = {}", errno)))),
                            Ok(host_file) => Ok(File {
                                handle: FileHandle::Host(HostHandle::new(host_file))
                            })
                        }
                    }
                }
            },
            Root::Initrd => {
                if let Some(initrd_file) = initrd::ROOT.find_file(stem.as_str()) {
                    Ok(File {
                        handle: FileHandle::Ram(RamHandle::new(initrd_file))
                    })
                } else {
                    Err(io::ErrorKind::NotFound.into())
                }
            },
            Root::Relative => unimplemented!(), // TODO
            Root::Unknown => Err(io::ErrorKind::NotFound.into())
        }
    }

    /// Attempts to open a file in write-only mode.
    pub fn create<'a, P: Into<Path<'a>>>(path: P) -> io::Result<File> {
        let path = path.into();
        let (root, stem) = path.split();

        match root {
            Root::Host => {
                let mut stem = String::from(stem.as_str());
                if stem.starts_with('/') {
                    stem.remove(0);
                }
                match path_to_cstr(&mut stem) {
                    None => Err(io::ErrorKind::NotFound.into()),
                    Some(filename) => {
                        match hosted::io::File::open(filename, hosted::io::FileMode::WriteBin) {
                            Err(errno) => Err(io::Error::new(io::ErrorKind::Other, FsError::new(format!("errno = {}", errno)))),
                            Ok(host_file) => Ok(File {
                                handle: FileHandle::Host(HostHandle::new(host_file))
                            })
                        }
                    }
                }
            },
            Root::Initrd => Err(io::ErrorKind::PermissionDenied.into()),
            Root::Relative => unimplemented!(), // TODO
            Root::Unknown => Err(io::ErrorKind::NotFound.into())
        }
    }
}

fn path_to_cstr<'a>(path: &'a mut String) -> Option<CStrRef<'a>> {
    path.push('\0');
    CStrRef::from_null_terminated_slice(path.as_bytes())
}

impl HostHandle {
    fn new(file: hosted::io::File) -> HostHandle {
        HostHandle { file }
    }
}

impl RamHandle {
    fn new(file: &'static initrd::File) -> RamHandle {
        RamHandle {
            file,
            cursor: 0
        }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.handle {
            FileHandle::Host(ref mut handle) => handle.read(buf),
            FileHandle::Ram(ref mut handle) => handle.read(buf)
        }
    }
}

impl Read for HostHandle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Read for RamHandle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = usize::min(buf.len(), self.file.contents.len() - self.cursor);
        buf[ .. len].copy_from_slice(&self.file.contents[self.cursor .. self.cursor + len]);
        self.cursor += len;
        Ok(len)
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self.handle {
            FileHandle::Host(ref mut handle) => handle.seek(pos),
            FileHandle::Ram(ref mut handle) => handle.seek(pos)
        }
    }
}

impl Seek for HostHandle {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Seek for RamHandle {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                self.cursor = usize::min(offset as usize, self.file.contents.len());
            },
            SeekFrom::End(offset) => {
                let file_len = self.file.contents.len();
                if offset >= 0 {
                    self.cursor = file_len;
                } else {
                    let neg_offset = (-offset) as usize;
                    if neg_offset > file_len {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                            FsError::new(String::from("attempted to seek to a negative file offset"))
                        ));
                    }
                    self.cursor = file_len - neg_offset;
                }
            },
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.cursor = usize::min(self.cursor + offset as usize, self.file.contents.len());
                } else {
                    let neg_offset = (-offset) as usize;
                    if neg_offset > self.cursor {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                            FsError::new(String::from("attempted to seek to a negative file offset"))
                        ));
                    }
                    self.cursor -= neg_offset;
                }
            }
        };
        Ok(self.cursor as u64)
    }
}

/// Returns a [`ReadDir`](./struct.ReadDir.html) object representing the contents of the directory
/// at the given path, if it exists and is accessible.
pub fn read_dir<'a, P: Into<Path<'a>>>(path: P) -> io::Result<ReadDir> {
    let path = path.into();
    let (root, stem) = path.split();

    match root {
        Root::Host => {
            Err(io::ErrorKind::PermissionDenied.into())
        },
        Root::Initrd => {
            if let Some(dir) = initrd::ROOT.find_dir(stem.as_str()) {
                Ok(ReadDir {
                    dir: Dir::Initrd(dir, 0, 0),
                    path: String::from(stem)
                })
            } else {
                Err(io::ErrorKind::NotFound.into())
            }
        },
        Root::Relative => unimplemented!(), // TODO
        Root::Unknown => Err(io::ErrorKind::NotFound.into())
    }
}

/// Represents the contents of a directory.
#[derive(Debug)]
pub struct ReadDir {
    dir: Dir,
    path: String
}

#[derive(Debug)]
enum Dir {
    Initrd(&'static initrd::Directory<'static>, usize, usize)
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.dir {
            Dir::Initrd(ref dir, ref mut dir_cursor, ref mut file_cursor) => {
                if *dir_cursor < dir.subdirs.len() {
                    *dir_cursor += 1;
                    Some(Ok(DirEntry {
                        path: self.path.clone() + "/" + dir.subdirs[*dir_cursor - 1].name,
                        metadata: Metadata {
                            file_type: FileType {
                                flags: FileTypeFlags::DIRECTORY
                            }
                        }
                    }))
                } else if *file_cursor < dir.files.len() {
                    *file_cursor += 1;
                    Some(Ok(DirEntry {
                        path: self.path.clone() + "/" + dir.files[*file_cursor - 1].name,
                        metadata: Metadata {
                            file_type: FileType {
                                flags: FileTypeFlags::FILE
                            }
                        }
                    }))
                } else {
                    None
                }
            }
        }
    }
}

/// Represents a single file or directory within a directory.
pub struct DirEntry {
    path: String,
    metadata: Metadata
}

impl DirEntry {
    /// Returns the path to this directory entry.
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Returns this directory entry's metadata.
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(self.metadata)
    }

    /// Returns the file type of this directory entry.
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(self.metadata.file_type())
    }

    /// Returns the name of this directory entry.
    pub fn file_name(&self) -> &str {
        &self.path[self.path.rfind('/').map(|x| x + 1).unwrap_or(0) .. ]
    }
}

/// Represents the metadata of a file or directory.
#[derive(Debug, Clone, Copy)]
pub struct Metadata {
    file_type: FileType
    // TODO: Add more metadata.
}

impl Metadata {
    /// Returns the file type indicated by these metadata.
    pub fn file_type(&self) -> FileType {
        self.file_type
    }
}

/// Represents a file type.
#[derive(Debug, Clone, Copy)]
pub struct FileType {
    flags: FileTypeFlags
}

impl FileType {
    /// Returns `true` if this file is actually a directory.
    pub fn is_dir(&self) -> bool {
        self.flags.contains(FileTypeFlags::DIRECTORY)
    }

    /// Returns `true` if this is a file.
    pub fn is_file(&self) -> bool {
        self.flags.contains(FileTypeFlags::FILE)
    }

    /// Returns `true` if this file is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.flags.contains(FileTypeFlags::SYMLINK)
    }
}

bitflags! {
    struct FileTypeFlags: u8 {
        const DIRECTORY = 0x01;
        const FILE      = 0x02;
        const SYMLINK   = 0x04;
    }
}

/// Represents an error that can arise from functions in this module.
#[derive(Debug)]
pub struct FsError {
    desc: String
}

impl FsError {
    /// Constructs a new error with the given description.
    // TODO: `desc` should have type i18n::Text.
    pub fn new(desc: String) -> FsError {
        FsError { desc }
    }
}

impl Error for FsError {}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc)
    }
}
