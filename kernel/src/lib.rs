/* Copyright (c) 2017-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#![deny(warnings, missing_docs)]

extern crate alloc;
#[cfg_attr(not(feature = "unit-test"), macro_use)] extern crate io;

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

    libphoenix::profiler,

    fs::File,
    i18n::Text,
    io::{Read, Write},
    scheduler::{Process, Thread},
};

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

#[cfg(target_machine = "qemu-virt")]
extern {
    static PROFILER_START_TIME_NANOSECS: u64;
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

    #[cfg(feature = "profiler")] {
        println!("Bootloader profile");
        println!("------------------");
        print_profile(time::SystemTime::from_raw_nanosecs(unsafe { PROFILER_START_TIME_NANOSECS }));
        println!();
    }

    // TODO: Instead of starting a shell in the kernel, run some programs and drivers.
    shell();

    let threads = Vec::new();
    scheduler::run(threads);
}

#[cfg(feature = "profiler")]
#[cfg(target_machine = "qemu-virt")]
fn print_profile(profiler_start_time: time::SystemTime) {
    let now = time::SystemTime::now();
    let nanos_elapsed = now.duration_since(profiler_start_time)
        .unwrap_or(core::time::Duration::ZERO)
        .as_nanos() as u64;
    let seconds_elapsed = nanos_elapsed as f64 / 1_000_000_000.0;

    for probe in profiler::probes() {
        let visits = probe.visits();
        println!("{}:{}:{} ({})", probe.file(), probe.line(), probe.column(), probe.scope());
        println!("Visits: {}", visits);
        println!("Throughput: {} visits/sec", probe.avg_throughput_hz());
        if let Some(latency) = probe.avg_latency_secs() {
            let total_time = latency * visits as f64;
            println!("Latency: {} sec", latency);
            println!("Total time consumed: {} sec ({:.2}%)", total_time, total_time * 100.0 / seconds_elapsed)
        }
        println!();
    }

    println!("Total time elapsed: {} sec", seconds_elapsed);
}

#[cfg(target_machine = "qemu-virt")]
fn shell() {
    profiler::reset();
    let mut profiler_start_time = time::SystemTime::now();

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
                    println!("help                    Shows this list of commands");
                    println!("list DIRECTORY          Lists all of the files and subdirectories in the given directory");
                    println!("parse FILE              Parses the given executable file and prints the image object for debugging");
                    println!("profile                 Shows the data gathered by the profiler.");
                    println!("profile reset           Resets the profiler.");
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
                Some("profile") => {
                    if let Some(word2) = split.next() {
                        if word2 == "reset" {
                            profiler::reset();
                            profiler_start_time = time::SystemTime::now();
                        } else {
                            println!("Unexpected word `{}`", word2);
                        }
                    } else {
                        print_profile(profiler_start_time);
                    }
                },
                Some("run") => {
                    if let Some(filename) = split.next() {
                        match File::open(filename) {
                            Ok(file) => {
                                match exec::read_exe(file) {
                                    Ok(image) => {
                                        let entry_point = image.entry_point;
                                        let process = Arc::new(Process::new(image, Vec::new()));
                                        match Thread::new(process, entry_point, 0, 0x0001_0000, 10) {
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
