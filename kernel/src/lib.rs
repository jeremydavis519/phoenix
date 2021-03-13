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

//! This crate and its dependencies comprise the Phoenix kernel.
//!
//! Much of this code in its early form comes from https://os.phil-opp.com/set-up-rust/. That blog should help with any Rust-related questions.

#![no_std]
#![feature(allocator_api)]
#![feature(const_fn)]
#![feature(untagged_unions)]

#![deny(warnings, missing_docs)]

extern crate alloc;
#[macro_use] extern crate bitflags; // TODO: We're only using this for a prototype.
#[cfg_attr(not(feature = "unit-test"), macro_use)] extern crate io;
#[macro_use] extern crate shared; // TODO: We're only using this for a prototype.

// Without this line, `rustc` thinks the `int` crate isn't needed because no Rust code uses anything
// from it. But then the linker complains because, of course, it really is needed.
extern crate int;

#[cfg(not(feature = "unit-test"))]
use {
    alloc::{
        vec,
        vec::Vec,
        sync::Arc
    },

    rlibc as _,

    fs::File,
    i18n::Text,
    io::{Read, Write},
    scheduler::Thread
};

#[allow(dead_code)]
mod gfx_prototype;

// TODO: Remove this temporary function.
/// Prints a single byte as a character.
#[no_mangle]
#[cfg(target_machine = "qemu-virt")]
pub extern fn putb(b: u8) {
    print!("{}", b as char);
}

// TODO: Remove this temporary function.
/// Prints a hexadecimal number.
#[no_mangle]
#[cfg(target_machine = "qemu-virt")]
pub extern fn putx(x: u64) {
    print!("{:#x}", x);
}

/// The entry point from the assembly bootloader.
#[no_mangle]
#[cfg(target_machine = "qemu-virt")]
pub extern fn kmain() -> ! {
    // TODO: Move these first output lines somewhere more appropriate.
    let version = *shared::KERNEL_VERSION;
    let homepage = *shared::KERNEL_HOMEPAGE;
    print!("{}", Text::PhoenixVersionHomepage(
        version,
        homepage.filter(|s| !s.is_empty())
    ));
    println!();

    // TODO: Move all of this initialization somewhere more appropriate. Everything should be done
    // lazily if possible.
    timers::init_per_cpu();

    // TODO: Instead of starting a shell in the kernel, run some programs and drivers.
    shell();

    let threads = Vec::new();
    scheduler::run(threads);
}

#[cfg(target_machine = "qemu-virt")]
fn shell() {
    println!("Temporary Phoenix kernel shell");
    println!("Type `help` for a list of commands.");
    loop {
        print!("\n> ");
        let mut command_buffer = [0u8; 256];
        let mut stdin = loop {
            match io::STDIN.try_lock() {
                Ok(lock) => break lock,
                Err(()) => core::hint::spin_loop()
            };
        };
        let mut index = 0;
        loop {
            let at_char_boundary = core::str::from_utf8(&command_buffer[ .. index]).is_ok();

            let mut buffer = [0u8; 1];
            if let Err(_) = stdin.read_exact(&mut buffer[ .. ]) {
                println!("\nError while reading input. Treating as a whole line.");
                break;
            }

            if at_char_boundary && (buffer[0] == b'\x08' || buffer[0] == b'\x7f') { // Backspace
                if index > 0 {
                    loop {
                        index -= 1;
                        if core::str::from_utf8(&command_buffer[ .. index]).is_ok() { break; }
                    }
                    let mut stdout = loop {
                        match io::STDOUT.try_lock() {
                            Ok(lock) => break lock,
                            Err(()) => core::hint::spin_loop()
                        };
                    };
                    let _ = stdout.write_all(&buffer[ .. ]);
                }
            } else if at_char_boundary && buffer[0] == b'\r' { // Return
                println!();
                break;
            } else if !at_char_boundary || buffer[0] >= 32 { // Don't store ASCII control characters
                if index < command_buffer.len() {
                    command_buffer[index] = buffer[0];
                    index += 1;
                    let mut stdout = loop {
                        match io::STDOUT.try_lock() {
                            Ok(lock) => break lock,
                            Err(()) => core::hint::spin_loop()
                        };
                    };
                    let _ = stdout.write_all(&buffer[ .. ]);
                }
            }
        }
        if let Ok(command) = core::str::from_utf8(&command_buffer[ .. index]) {
            let mut split = command.split_whitespace();
            match split.next() {
                Some("help") => {
                    println!("contents [FILE [...]]   Prints out the contents of the given text files concatenated");
                    println!("devices                 Prints out the device tree");
                    println!("gfx                     Launches the prototype graphics card driver");
                    println!("gfx-displays            Shows information about all the connected displays");
                    println!("help                    Shows this list of commands");
                    println!("list DIRECTORY          Lists all of the files and subdirectories in the given directory");
                    println!("parse FILE              Parses the given executable file and prints the image object for debugging");
                    println!("run FILE                Runs the given executable file in userspace");
                    println!("shutdown                Shuts down the computer");
                    println!("time                    Shows (`time`) or sets (`time <hh:mm:ss>`) the current time");
                },
                Some("contents") => {
                    for filename in split {
                        match File::open(filename) {
                            Ok(mut file) => {
                                let mut buf = [0u8; 32];
                                loop {
                                    match file.read(&mut buf) {
                                        Ok(0)   => break,
                                        Ok(len) => {
                                            let mut stdout = loop {
                                                match io::STDOUT.try_lock() {
                                                    Ok(lock) => break lock,
                                                    Err(()) => core::hint::spin_loop()
                                                };
                                            };
                                            let _ = stdout.write_all(&buf[ .. len]);
                                        },
                                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {},
                                        Err(e)  => {
                                            println!("Error reading file `{}`: {}", filename, e);
                                        }
                                    };
                                }
                                println!();
                            },
                            Err(e) => println!("Error opening file `{}`: {}", filename, e)
                        };
                    }
                },
                Some("devices") => {
                    println!("{:#?}", *devices::DEVICES);
                },
                Some("gfx") => {
                    for i in 0 .. 32 {
                        unsafe {
                            let addr = 0x0a00_0000 + i * 0x0200;
                            let ptr = addr as *const u32;
                            if *ptr == 0x74726976 && *ptr.add(1) == 1 && *ptr.add(2) == 16 {
                                gfx_prototype::main(&gfx_prototype::Device { base_addr: gfx_prototype::Address::Mmio(addr) } as *const _);
                                break;
                            }
                        }
                    }
                },
                Some("gfx-displays") => {
                    gfx_prototype::EXECUTOR.spawn(async {
                        println!("{:#?}", gfx_prototype::DisplayInfo::all().await);
                    }).execute_blocking();
                },
                Some("list") => {
                    if let Some(dirname) = split.next() {
                        if let Some(_) = split.next() {
                            println!("Too many arguments to `list`");
                        } else {
                            match fs::read_dir(dirname) {
                                Ok(dir) => {
                                    for entry in dir {
                                        match entry {
                                            Ok(entry) => {
                                                match entry.file_type() {
                                                    Ok(file_type) => {
                                                        if file_type.is_dir() {
                                                            println!("{}/", entry.file_name());
                                                        } else if file_type.is_file() {
                                                            println!("{}", entry.file_name());
                                                        } else if file_type.is_symlink() {
                                                            println!("{} (symlink)", entry.file_name());
                                                        }
                                                    },
                                                    Err(e) => println!("Error getting entry type: {}", e)
                                                };
                                            },
                                            Err(e) => println!("Error reading entry: {}", e)
                                        };
                                    }
                                },
                                Err(e) => println!("Error opening directory `{}`: {}", dirname, e)
                            };
                        }
                    } else {
                        println!("Expected a path after `list`");
                    }
                },
                Some("parse") => {
                    if let Some(filename) = split.next() {
                        match File::open(filename) {
                            Ok(file) => {
                                match exec::read_exe(file) {
                                    Ok(image) => println!("{:#x?}", image),
                                    Err(e)    => println!("Error parsing file `{}`: {}", filename, e)
                                };
                            },
                            Err(e) => println!("Error opening file `{}`: {}", filename, e)
                        };
                    } else {
                        println!("Expected a file after `parse`");
                    }
                },
                Some("run") => {
                    if let Some(filename) = split.next() {
                        match File::open(filename) {
                            Ok(file) => {
                                match exec::read_exe(file) {
                                    Ok(image) => {
                                        let image = Arc::new(image);
                                        let entry_point = image.entry_point;
                                        match Thread::new(image, entry_point, 0x0001_0000, 10) {
                                            Ok(thread) => scheduler::run(vec![thread]),
                                            Err(e) => println!("Error creating the thread: {}", e)
                                        };
                                    },
                                    Err(e) => println!("Error parsing file `{}`: {}", filename, e)
                                };
                            },
                            Err(e) => println!("Error opening file `{}`: {}", filename, e)
                        };
                    } else {
                        println!("Expected a file after `parse`");
                    }
                },
                Some("shutdown") => {
                    if let Some(_) = split.next() {
                        println!("Too many arguments to `shutdown`");
                    } else {
                        hosted::exit(0);
                        println!("Tried to quit and couldn't. Maybe this isn't an emulator?");
                    }
                },
                Some("time") => {
                    if let Some(new_time) = split.next() {
                        // Set time
                        if let Some(_) = split.next() {
                            println!("Too many arguments to `time`");
                        } else {
                            let mut time_components = new_time.split(':');
                            if let Some(hour) = time_components.next() {
                                if let Some(minute) = time_components.next() {
                                    if let Some(second) = time_components.next() {
                                        if let Some(_) = time_components.next() {
                                            println!("Too many time components");
                                        } else {
                                            if let (Ok(hour), Ok(minute), Ok(second)) =
                                                    (u64::from_str_radix(hour, 10), u64::from_str_radix(minute, 10), u64::from_str_radix(second, 10)) {
                                                if hour < 24 && minute < 60 && second < 60 {
                                                    // Get the beginning of the day so we don't change
                                                    // the day in addition to the time.
                                                    let old_now = time::SystemTime::now();
                                                    let old_time_since_epoch = old_now.duration_since(time::SystemTime::UNIX_EPOCH).unwrap();
                                                    let old_secs_since_epoch = old_time_since_epoch.as_secs();
                                                    let today_secs_since_epoch = old_secs_since_epoch - old_secs_since_epoch % (60 * 60 * 24);

                                                    let new_time_since_epoch = time::Duration::from_secs(second + minute * 60 + hour * 3600
                                                        + today_secs_since_epoch);
                                                    let new_now = time::SystemTime::UNIX_EPOCH + new_time_since_epoch;
                                                    time::SystemTime::set_now(new_now);
                                                } else {
                                                    println!("Given time is out of bounds");
                                                }
                                            } else {
                                                println!("Time must be given as 3 integers");
                                            }
                                        }
                                    } else {
                                        println!("Missing seconds");
                                    }
                                } else {
                                    println!("Missing minutes");
                                }
                            } else {
                                println!("Missing hours");
                            }
                        }
                    } //else {
                        // Get time
                        let current_time = time::SystemTime::now();
                        let time_since_epoch = current_time.duration_since(time::SystemTime::UNIX_EPOCH).unwrap();
                        let total_seconds = time_since_epoch.as_secs();
                        let seconds = total_seconds % 60;
                        let minutes = (total_seconds / 60) % 60;
                        let hours = (total_seconds / 3600) % 24;
                        println!("{}:{:02}:{:02} (UNIX timestamp: {})", hours, minutes, seconds, total_seconds);
                    //}
                },
                Some(command) => println!("Unrecognized command `{}`. Type `help` for a list of accepted commands.", command),
                None => {}
            };
        } else {
            println!("Error: Invalid UTF-8");
        }
    }
}
