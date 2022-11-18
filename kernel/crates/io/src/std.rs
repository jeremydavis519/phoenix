/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module contains things that would normally be defined in `std::io` if we could use `std`.

use core::fmt;
#[cfg(target_machine = "qemu-virt")]
use {
    locks::Mutex,
    shared::ffi::CStrRef
};

use i18n::Text;

/// Prints the given string with format arguments, followed by a newline.
#[macro_export]
macro_rules! println {
    ($($fmt:expr)?) => { $crate::print!(concat!($($fmt,)? "\n")) };
    ($fmt:expr $(, $arg:expr)+) => { $crate::print!(concat!($fmt, "\n") $(, $arg)+) };
}

/// Prints the given string with format arguments (not followed by a newline).
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::_print(format_args!($($arg)*)) };
}

/// Prints the given string with format arguments, followed by a newline, if the code was built in
/// debug mode. Note: Any side-effects associated with the arguments will take place even in
/// release mode.
#[macro_export]
macro_rules! printlndebug {
    ($($fmt:expr)?) => { $crate::printdebug!(concat!($($fmt,)? "\n")) };
    ($fmt:expr $(, $arg:expr)+) => { $crate::printdebug!(concat!($fmt, "\n") $(, $arg)+) };
}

/// Prints the given string with format arguments (not followed by a newline) if the code was built
/// in debug mode. Note: Any side-effects associated with the arguments will take place even in
/// release mode.
#[macro_export]
macro_rules! printdebug {
    ($($fmt:expr)?) => {
        #[cfg(debug_assertions)] $crate::print!($($fmt)?);
        #[cfg(not(debug_assertions))] {
            $($fmt;)?
        }
    };

    ($fmt:expr $(, $arg:expr)+) => {
        #[cfg(debug_assertions)] $crate::print!($fmt $(, $arg)+);
        #[cfg(not(debug_assertions))] {
            $fmt;
            $($arg;)+
        }
    };
}

#[cfg(target_machine = "qemu-virt")]
lazy_static! {
    unsafe {
        /// Standard output
        pub static ref STDOUT: &'static Mutex<crate::serial::SerialWriter> = &(*crate::serial::UART0).writer;
        /// Standard input
        pub static ref STDIN: &'static Mutex<crate::serial::SerialReader> = &(*crate::serial::UART0).reader;
    }
}

#[doc(hidden)]
#[cfg(target_machine = "qemu-virt")]
pub fn _print(args: fmt::Arguments) {
    // FIXME: Redesign the kernel's output method to avoid this blocking.
    loop {
        if let Ok(mut stdout) = STDOUT.try_lock() {
            let _ = stdout.write_fmt(args);
            break;
        }
        core::hint::spin_loop();
    }
}
#[doc(hidden)]
#[cfg(target_arch = "x86_64")]
pub fn _print(_args: fmt::Arguments) { unimplemented!() }

/// This function allows assembly code to write strings to the standard output. `text` is a pointer
/// to the null-terminated UTF-8 string to print.
#[no_mangle]
#[cfg(target_machine = "qemu-virt")]
unsafe extern fn puts(text: *const u8) {
    if text.is_null() {
        panic!("null pointer passed to `puts`");
    }
    print!("{}", CStrRef::from_ptr(text).as_str().expect("invalid UTF-8 string passed to `puts`"));
}

/// Allows reading from a stream of bytes, wherever that stream came from. For more information,
/// see [the standard library documentation](https://doc.rust-lang.org/std/io/trait.Read.html).
pub trait Read {
    /// Reads bytes into the given buffer until either it is filled or an error occurs.
    ///
    /// # Returns
    /// `Ok(x)` after having read `x` bytes into the buffer. `Ok(0)` is possible.
    ///
    /// `Err` if an error prevents reading even one byte.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Reads exactly `buf.len()` bytes into the given buffer.
    ///
    /// # Returns
    /// `Ok` after having filled the buffer with no errors.
    ///
    /// `Err` if an error occurs. The contents of the buffer are undefined in this case.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut index = 0;
        while index < buf.len() {
            match self.read(&mut buf[index .. ]) {
                Ok(0)                                            => return Err(ErrorKind::UnexpectedEof.into()),
                Ok(bytes_read)                                   => index += bytes_read,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {},
                Err(e)                                           => return Err(e)
            };
        }
        Ok(())
    }

    /// Returns an iterator over the bytes in this stream.
    fn bytes(self) -> Bytes<Self>
            where Self: Sized {
        Bytes {
            reader: self
        }
    }

    /// Returns an iterator over the first `limit` bytes in this stream.
    fn take(self, limit: u64) -> Take<Self>
            where Self: Sized {
        Take {
            reader: self,
            bytes_left: limit
        }
    }

    // TODO: Some methods are missing. Add them as needed.
}

/// An iterator over a byte stream.
#[derive(Debug)]
pub struct Bytes<T: Read> {
    reader: T
}

impl<T: Read> Iterator for Bytes<T> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = [0u8; 1];
        loop {
            match self.reader.read(&mut buf) {
                Ok(0) => return None,
                Ok(1) => return Some(Ok(buf[0])),
                Ok(_) => panic!("{}", Text::ReadPastBuffer),
                Err(e) if e.kind() == ErrorKind::Interrupted => {},
                Err(e) => return Some(Err(e)),
            };
        }
    }
}

/// An iterator over a certain number of bytes at the beginning of a byte stream.
#[derive(Debug)]
pub struct Take<T: Read> {
    reader: T,
    bytes_left: u64
}

impl<T: Read> Take<T> {
    // TODO: Add methods as needed.
}

impl<T: Read> Read for Take<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.bytes_left == 0 {
            Ok(0)
        } else {
            let buf = if self.bytes_left <= usize::max_value() as u64 {
                let len = usize::min(buf.len(), self.bytes_left as usize);
                &mut buf[ .. len]
            } else {
                buf // `&mut buf[ .. usize::min(buf.len(), usize::max_value())]` is equivalent to `&mut buf[ .. buf.len()]`.
            };
            match self.reader.read(buf) {
                Ok(0) => Ok(0),
                Ok(x) => {
                    self.bytes_left -= x as u64;
                    Ok(x)
                },
                e @ Err(_) => e
            }
        }
    }
}

/// Allows writing to a stream of bytes, wherever that stream came from. For more information, see
/// [the standard library documentation](https://doc.rust-lang.org/std/io/trait.Write.html).
pub trait Write {
    /// Writes bytes from the given buffer to the stream until the entire buffer is written or an
    /// error occurs.
    ///
    /// # Returns
    /// `Ok(x)` after writing `x` bytes. `Ok(0)` is possible.
    ///
    /// `Err` if an error prevents writing even one byte.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flushes the stream, in case this is a buffered writer.
    fn flush(&mut self) -> Result<()>;

    /// Writes all of the contents of the given buffer to the stream.
    ///
    /// # Returns
    /// `Ok` after writing the whole buffer without errors.
    ///
    /// `Err` if an error occurs. The state of the stream is undefined in this case.
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let mut index = 0;
        while index < buf.len() {
            match self.write(&buf[index .. ]) {
                Ok(0) => return Err(ErrorKind::WriteZero.into()),
                Ok(x) => index += x,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {},
                Err(e) => return Err(e)
            }
        }
        Ok(())
    }

    /// Writes a formatted string to the stream. This shouldn't be called directly. Use the
    /// `write!` macro instead.
    ///
    /// # Returns
    /// `Ok` after writing the whole string without errors.
    ///
    /// `Err` if an error occurs. The state of the stream is undefined in this case.
    fn write_fmt(&mut self, fmt: fmt::Arguments) -> Result<()> {
        // This idea comes directly from the standard library's source code. It's a shim to avoid
        // discarding `io` errors.
        struct Adaptor<'a, T: ?Sized+'a> {
            inner: &'a mut T,
            error: Result<()>
        }

        impl<'a, T: Write+?Sized> fmt::Write for Adaptor<'a, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Err(e);
                        Err(fmt::Error)
                    }
                }
            }
        }

        let mut writer = Adaptor { inner: self, error: Ok(()) };
        match fmt::write(&mut writer, fmt) {
            Ok(()) => Ok(()),
            Err(_) => {
                if writer.error.is_err() {
                    writer.error
                } else {
                    Err(ErrorKind::Other.into())
                }
            }
        }
    }
}

/// Any type that supports a cursor that can seek over a byte stream should implement this trait.
pub trait Seek {
    /// Seeks to the given position in the stream.
    ///
    /// # Returns
    /// `Ok(x)`, where `x` is the new position, measured in bytes from the start. Calling this
    /// later with `pos` set to `SeekFrom::Start(x)` will seek to the same location again.
    ///
    /// # Possible errors
    /// Seeking to a position before the beginning of the stream results in an `InvalidInput`
    /// error.
    ///
    /// Seeking past the end is allowed but results in implementation-defined behavior.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
}

/// Represents a point in a file to which to seek.
#[derive(Debug)]
pub enum SeekFrom {
    /// Seeks the given number of bytes forward from the start of the file.
    Start(u64),
    /// Seeks the given number of bytes forward from the end of the file. (This is expected to be
    /// negative, which leads to seeking backward.)
    End(i64),
    /// Seeks the given number of bytes forward from the current position. (This can be negative to
    /// seek backward.)
    Current(i64)
}

pub use self::error::{Error, ErrorKind};

/// Since all I/O functions return the same error type, we use this type to avoid writing
/// `Result<T, io::Error>` everywhere.
pub type Result<T> = core::result::Result<T, Error>;

mod error {
    use {
        alloc::{
            alloc::AllocError,
            boxed::Box
        },
        core::fmt,
        i18n::Text,
        error::Error as ErrorTrait
    };

    /// This type is somewhat simpler than the one in the standard library. That one has to be able to
    /// represent "OS" errors, but this _is_ the OS.
    #[derive(Debug)]
    pub struct Error {
        repr: Repr
    }

    #[derive(Debug)]
    enum Repr {
        Simple(ErrorKind),
        Custom(ErrorKind, Box<dyn ErrorTrait+Send+Sync>),
        CustomOom(ErrorKind) // Like the `Custom` variant, but used if we run out of memory while making it.
    }

    impl Error {
        /// Makes a new error of the given kind and with the given inner error.
        pub fn new<E>(kind: ErrorKind, error: E) -> Error
                where E: ErrorTrait+Send+Sync+'static {
            match Box::try_new(error) {
                Ok(e) => Error { repr: Repr::Custom(kind, e as Box<dyn ErrorTrait+Send+Sync>) },
                Err(AllocError) => Error { repr: Repr::CustomOom(kind) }
            }
        }

        /// Returns a reference to the inner error, if any.
        pub fn get_ref(&self) -> Option<&(dyn ErrorTrait+Send+Sync+'static)> {
            match self.repr {
                Repr::Simple(_) => None,
                Repr::Custom(_, ref inner) => Some(&**inner),
                Repr::CustomOom(_) => None
            }
        }

        /// Returns a mutable reference to the inner error, if any.
        pub fn get_mut(&mut self) -> Option<&mut (dyn ErrorTrait+Send+Sync+'static)> {
            match self.repr {
                Repr::Simple(_) => None,
                Repr::Custom(_, ref mut inner) => Some(&mut **inner),
                Repr::CustomOom(_) => None
            }
        }

        /// Consumes this error and returns the inner error, if any.
        pub fn into_inner(self) -> Option<Box<dyn ErrorTrait+Send+Sync>> {
            match self.repr {
                Repr::Simple(_) => None,
                Repr::Custom(_, inner) => Some(inner),
                Repr::CustomOom(_) => None
            }
        }

        /// Returns the kind of error this is.
        pub fn kind(&self) -> ErrorKind {
            match self.repr {
                Repr::Simple(kind) => kind,
                Repr::Custom(kind, _) => kind,
                Repr::CustomOom(kind) => kind
            }
        }
    }

    impl ErrorTrait for Error {}

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match &self.repr {
                Repr::Simple(kind) => write!(f, "{}", kind),
                Repr::Custom(kind, e) => write!(f, "{}: {}", kind, &**e),
                Repr::CustomOom(kind) => write!(f, "{}: (ran out of memory while allocating space for the error message)", kind)
            }
        }
    }

    impl From<ErrorKind> for Error {
        fn from(kind: ErrorKind) -> Error {
            Error { repr: Repr::Simple(kind) }
        }
    }

    /// Communicates which kind of I/O error has been returned, just like in the standard library.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[non_exhaustive]
    pub enum ErrorKind {
        /// The resource was not found.
        NotFound,
        /// The caller does not have permission to access the resource.
        PermissionDenied,
        /// The remote server refused the connection.
        ConnectionRefused,
        /// The remote server reset the connection.
        ConnectionReset,
        /// The remote server terminated the connection early.
        ConnectionAborted,
        /// The network operation failed because it was not connected.
        NotConnected,
        /// A socket could not be bound because its address was already in use.
        AddrInUse,
        /// The given interface address didn't exist or wasn't local.
        AddrNotAvailable,
        /// A pipe was closed.
        BrokenPipe,
        /// The resource already exists.
        AlreadyExists,
        /// The operation would need to block but was requested not to block.
        WouldBlock,
        /// A parameter was incorrect.
        InvalidInput,
        /// The data encountered were invalid for this operation.
        InvalidData,
        /// The operation was canceled because it took too long.
        TimedOut,
        /// A call to `write` returned `Ok(0)`.
        WriteZero,
        /// The operation was interrupted. It can be retried.
        Interrupted,
        /// Any I/O error not in this list.
        Other,
        /// Found the end of a file too early.
        UnexpectedEof
    }

    impl fmt::Display for ErrorKind {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", match self {
                ErrorKind::NotFound          => Text::IoErrNotFound,
                ErrorKind::PermissionDenied  => Text::IoErrPermissionDenied,
                ErrorKind::ConnectionRefused => Text::IoErrConnectionRefused,
                ErrorKind::ConnectionReset   => Text::IoErrConnectionReset,
                ErrorKind::ConnectionAborted => Text::IoErrConnectionAborted,
                ErrorKind::NotConnected      => Text::IoErrNotConnected,
                ErrorKind::AddrInUse         => Text::IoErrAddrInUse,
                ErrorKind::AddrNotAvailable  => Text::IoErrAddrNotAvailable,
                ErrorKind::BrokenPipe        => Text::IoErrBrokenPipe,
                ErrorKind::AlreadyExists     => Text::IoErrAlreadyExists,
                ErrorKind::WouldBlock        => Text::IoErrWouldBlock,
                ErrorKind::InvalidInput      => Text::IoErrInvalidInput,
                ErrorKind::InvalidData       => Text::IoErrInvalidData,
                ErrorKind::TimedOut          => Text::IoErrTimedOut,
                ErrorKind::WriteZero         => Text::IoErrWriteZero,
                ErrorKind::Interrupted       => Text::IoErrInterrupted,
                ErrorKind::Other             => Text::IoErrOther,
                ErrorKind::UnexpectedEof     => Text::IoErrUnexpectedEof
            })
        }
    }
}

/// This trait isn't actually in the standard library, but this was a convenient place to put it.
/// It does nothing except allow trait objects to be made that implement both `Read` and `Seek`.
pub trait ReadSeek: Read+Seek {}
impl<T: Read+Seek> ReadSeek for T {}
