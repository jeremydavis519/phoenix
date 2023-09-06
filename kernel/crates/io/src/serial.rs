/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module provides an interface for reading from and writing to serial ports.

// TODO: Refactor this whole module into a more object-oriented style (e.g. make `uart0` into an object).
//       This change will allow more flexibility in the number and types of serial ports.

use locks::Mutex;

#[cfg(target_machine = "qemu-virt")]
use shared::wait_for_event;
#[cfg(target_machine = "qemu-virt")]
use crate::std::{self, Read, Write};

/// Contains the writer and reader for a particular serial interface. Only one of each may exist for each serial port, since
/// each port has only one data channel in each direction.
pub struct SerialPort {
    /// Allows writing bytes to the serial port.
    pub writer: Mutex<SerialWriter>,
    /// Allows reading bytes from the serial port.
    pub reader: Mutex<SerialReader>
}

#[cfg(target_machine = "qemu-virt")]
lazy_static! {
    unsafe {
        /// The primary serial port.
        pub static ref UART0: SerialPort = {
            init(115_200);
            SerialPort {
                writer: Mutex::new(SerialWriter::new()),
                reader: Mutex::new(SerialReader::new())
            }
        };
    }
}

/// Initializes the serial port.
///
/// # Safety
/// Requires that no other thread use the serial port at the same time. Guaranteed by locking uart0::MMIO.
#[cfg(target_machine = "qemu-virt")]
fn init(baud: u32) {
    let uart = uart0::MMIO.try_lock().unwrap();

    unsafe {
        // TODO: Fully understand what's actually happening here. Some bitflags structs might be useful.
        // Disable UART0 (to reset it?).
        (*uart.index(uart0::Regs::CR as usize)).write(0x0000_0000);

        // Clear all pending interrupts from UART0.
        (*uart.index(uart0::Regs::ICR as usize)).write(0x7ff);

        // The baud rate is set using two registers, for the integer part and the fractional part of a divider.
        // Divider = uart0::CLOCK / (16 * baud)
        // Fractional part register = (fractional part * 64) + 0.5 = (remainder / (16 * baud) * 128 + 1) / 2
        //                          = (8 * remainder + baud) / (2 * baud)
        let divider = uart0::CLOCK / (16 * baud);
        let remainder = uart0::CLOCK % (16 * baud);
        (*uart.index(uart0::Regs::IBRD as usize)).write(divider);
        (*uart.index(uart0::Regs::FBRD as usize)).write((8 * remainder + baud) / (2 * baud));

        // Enable the FIFO and 8-bit data transmission (1 stop bit, no parity).
        (*uart.index(uart0::Regs::LCRH as usize)).write((1 << 4) | (1 << 5) | (1 << 6));

        // Mask all interrupts.
        (*uart.index(uart0::Regs::IMSC as usize)).write((1 << 1) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) |
            (1 << 9) | (1 << 10));

        // Enable UART0, the receive & transfer part of UART.
        (*uart.index(uart0::Regs::CR as usize)).write((1 << 0) | (1 << 8) | (1 << 9));
    }
}

/// Handles writing text to the serial port. Supports the write! macro.
#[derive(Debug, Clone, Copy)]
pub struct SerialWriter {}

/// Handles reading text from the serial port.
#[derive(Debug, Clone, Copy)]
pub struct SerialReader {}

#[cfg(target_machine = "qemu-virt")]
impl SerialWriter {
    /// Creates a new SerialWriter. This is marked as unsafe because making more than one for the same port causes
    /// the whole interface for that port to be unsafe.
    pub const unsafe fn new() -> SerialWriter {
        SerialWriter {}
    }

    /// Writes a byte to the serial port.
    ///
    /// # Safety
    /// Depends on exclusive write access to the serial port. Guaranteed by having a singleton behind a mutex.
    ///
    /// # Parameters
    /// * b: The byte to write.
    ///
    /// # Example
    /// ```
    /// self.try_putb(0x80)?; // Writes 0x80
    /// ```
    fn try_putb(&mut self, b: u8) -> Result<(), ()> {
        let output = b as u32;
        if let Ok(uart) = uart0::MMIO.try_lock() {
            unsafe {
                // Wait for the serial port to be ready to transmit before sending the byte.
                while ((*uart.index(uart0::Regs::FR as usize)).read() & (1 << 5)) != 0 { wait_for_event(); }
                (*uart.index(uart0::Regs::DR as usize)).write(output);
            }
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(target_machine = "qemu-virt")]
impl Write for SerialWriter {
    /// Writes the given byte buffer to the serial port.
    ///
    /// # Parameters
    /// s: The string to write.
    ///
    /// # Example
    /// ```
    /// let writer = SerialWriter {};
    /// writer.write_str("Hello, world!"); // Writes "Hello, world!"
    /// ```
    fn write(&mut self, buf: &[u8]) -> std::Result<usize> {
        for (index, &b) in buf.iter().enumerate() {
            if self.try_putb(b).is_err() {
                if index == 0 {
                    return Err(std::ErrorKind::Interrupted.into());
                } else {
                    return Ok(index);
                }
            }
        }
        Ok(buf.len())
    }

    /// Flushes the buffer to the serial port. We don't have a buffer, so this is a no-op.
    fn flush(&mut self) -> std::Result<()> { Ok(()) }
}

#[cfg(target_machine = "qemu-virt")]
impl SerialReader {
    /// Creates a new SerialReader. This is marked as unsafe because making more than one for the same port causes
    /// the whole interface for that port to be unsafe.
    pub const unsafe fn new() -> SerialReader {
        SerialReader {}
    }

    /// Reads a byte from the serial port.
    ///
    /// # Safety
    /// Depends on exclusive read access to the serial port. Guaranteed by having a singleton behind a mutex.
    ///
    /// # Returns
    /// The byte that was read.
    fn try_getb(&mut self) -> Result<u8, ()> {
        if let Ok(uart) = uart0::MMIO.try_lock() {
            unsafe {
                // Wait for the FIFO to have a byte before trying to receive it.
                while ((*uart.index(uart0::Regs::FR as usize)).read() & (1 << 4)) != 0 { wait_for_event(); }
                Ok((*uart.index(uart0::Regs::DR as usize)).read() as u8)
            }
        } else {
            Err(())
        }
    }
}

#[cfg(target_machine = "qemu-virt")]
impl Read for SerialReader {
    /// Reads from the serial port into the given byte buffer.
    ///
    /// Parameters
    /// s: The string to write.
    ///
    /// Returns
    /// The number of bytes actually read.
    fn read(&mut self, buf: &mut [u8]) -> std::Result<usize> {
        for (index, buffered) in buf.iter_mut().enumerate() {
            match self.try_getb() {
                Ok(b) => *buffered = b,
                Err(()) => {
                    // TODO: If the serial device was disconnected, return an error.
                    if index == 0 {
                        return Err(std::ErrorKind::Interrupted.into());
                    } else {
                        return Ok(index);
                    }
                }
            }
        }
        Ok(buf.len())
    }
}

// A collection of constants related to the first serial port
mod uart0 {
    #[cfg(not(target_arch = "x86_64"))]
    use {
        alloc::alloc::AllocError,
        volatile::Volatile,
        locks::Mutex,
        i18n::Text,
        memory::{
            allocator::AllMemAlloc,
            phys::block::Mmio
        }
    };

    // The serial port's clock speed
    #[cfg(not(target_arch = "x86_64"))]
    pub const CLOCK: u32 = 3_000_000;

    // MMIO block
    #[cfg(any(target_machine = "raspi1", target_machine = "raspi2", target_machine = "raspi3"))] mod mmio {
        pub static MMIO_BASE: usize = GPIO_MMIO_BASE + 0x1000;
        pub static MMIO_SIZE: usize = 0x1000; // TODO: Verify this size.
    } #[cfg(target_machine = "qemu-virt")] mod mmio {
        pub static MMIO_BASE: usize = 0x0900_0000;
        pub static MMIO_SIZE: usize = 0x1000;
    }
    #[cfg(not(feature = "unit-test"))]
    use self::mmio::*;
    #[cfg(target_machine = "qemu-virt")]
    lazy_static! {
        unsafe {
            pub static ref MMIO: Mutex<Mmio<Volatile<u32>>> = {
                let result = AllMemAlloc.mmio_mut(MMIO_BASE, MMIO_SIZE);
                match result {
                    Ok(x) => Mutex::new(x),
                    Err(AllocError) => panic!("{}", Text::Uart0CouldntReserveMmio)
                }
            };
        }
    }

    // Register addresses (look them up online for more information)
    // TODO: Should some of these be read-only (i.e. not mutable)?
    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
    pub enum Regs {
        DR     = 0x00 / 4,
        RSRECR = 0x04 / 4,
        FR     = 0x18 / 4,
        ILPR   = 0x20 / 4,
        IBRD   = 0x24 / 4, // Integer Baud Rate Divider
        FBRD   = 0x28 / 4, // Fractional Baud Rate Divider
        LCRH   = 0x2C / 4,
        CR     = 0x30 / 4,
        IFLS   = 0x34 / 4,
        IMSC   = 0x38 / 4,
        RIS    = 0x3C / 4,
        MIS    = 0x40 / 4,
        ICR    = 0x44 / 4,
        DMACR  = 0x48 / 4,
        ITCR   = 0x80 / 4,
        ITIP   = 0x84 / 4,
        ITOP   = 0x88 / 4,
        TDR    = 0x8C / 4
    }
}
