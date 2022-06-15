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

//! This program is the Phoenix operating system's driver for the VirtIO GPU.
//!
//! # Required permissions:
//! * own device mmio/virtio-16

#![no_std]
#![deny(/*warnings, */missing_docs)]

#![feature(allocator_api)]
#![feature(default_alloc_error_handler)]
#![feature(inline_const)]
#![feature(start)]

extern crate alloc;

use {
    core::{
        arch::asm,
        fmt::Write,
        mem,
        slice
    },
    bitflags::bitflags,
    libphoenix::{
        future::SysCallExecutor,
        profiler,
        syscall
    },
    libdriver::Device,
    virtio::{
        DeviceEndian, DeviceDetails, GenericFeatures,
        virtqueue::future::Executor
    },
    self::api::*,
    self::msg::Rectangle
};

mod api;
mod msg;

const DEVICE_TYPE_GPU: u32 = 16;
const MAX_SCANOUTS: usize = 16;

fn main() {
    SysCallExecutor::new()
        .spawn(async {
            let mut kernel_profile = profiler::kernel_probes().await;
            syscall::time_reset_kernel_profile();
            let start_time_nanos = syscall::time_now_unix_nanos();

            let device = Device::claim("mmio/virtio-16").await
                .expect("no VirtIO GPU found");
            run_driver(kernel_profile, start_time_nanos, device);
        })
        .block_on_all();
}

fn run_driver<'a, I>(kernel_profile: I, start_time_nanos: u64, device: Device<'_>)
        where I: Iterator<Item = profiler::ProbeRef<'a>> {
    let mut device_details = match virtio::init(
            &device,
            DEVICE_TYPE_GPU,
            ConfigurationSpace::SIZE,
            QueueIndex::Count as u32,
            Features::empty().bits(),
            (Features::ANY_LAYOUT | Features::VERSION_1 | Features::ORDER_PLATFORM).bits()
    ) {
        Ok(x) => x,
        Err(e) => panic!("failed to initialize the VirtIO GPU: {}", e)
    };

    let virtqueues = device_details.virtqueues();
    let control_q = &virtqueues[QueueIndex::Control as usize];
    let cursor_q = &virtqueues[QueueIndex::Cursor as usize];

    let config_space = ConfigurationSpace::new(&mut device_details);

    Executor::new()
        .spawn(async {
            let display_info = api::DisplayInfo::one(control_q, 0).await
                .expect("failed to retrieve display info");
            assert!(display_info.flags.contains(DisplayInfoFlags::ENABLED));

            let mut fb = Image::new(Some(control_q), 0, 1, 32, display_info.rect.width, display_info.rect.height).await
                .expect("failed to create framebuffer");
            set_display_framebuffer(control_q, 0, &fb).await
                .expect("failed to set display framebuffer");

            fb.set_colors(0, &[Color(0xffff00ff)]);
            fb.draw_rect_region(Rectangle {
                x: 0, y: 0,
                width: 320, height: 240
            }, Tile(0));
            fb.set_colors(0, &[Color(0xffffff00)]);
            fb.draw_rect_region(Rectangle {
                x: 320, y: 240,
                width: 320, height: 240
            }, Tile(0));
            fb.send_all().await
                .expect("failed to send the framebuffer to the host");
            fb.flush_all().await
                .expect("failed to flush the framebuffer");

            print_profile(kernel_profile, start_time_nanos);

            let _ = writeln!(KernelWriter, "GPU test done");
            loop {}
        })
        .block_on_all();

    // Event loop
    // TODO: Should this be encapsulated in a library call?
    loop {
        // TODO: Handle events.
        return;
    }
}

fn print_profile<'a, I>(profile: I, start_time_nanos: u64) where I: Iterator<Item = profiler::ProbeRef<'a>> {
    let now_nanos = syscall::time_now_unix_nanos();
    let seconds_elapsed = now_nanos.saturating_sub(start_time_nanos) as f64 / 1_000_000_000.0;

    for probe in profile {
        let visits = probe.visits();
        let _ = writeln!(KernelWriter, "{}:{}:{}", probe.file(), probe.line(), probe.column());
        let _ = writeln!(KernelWriter, "Visits: {}", visits);
        let _ = writeln!(KernelWriter, "Throughput: {} visits/sec", probe.avg_throughput_hz());
        if let Some(latency) = probe.avg_latency_secs() {
            let total_time = latency * visits as f64;
            let _ = writeln!(KernelWriter, "Latency: {} sec", latency);
            let _ = writeln!(KernelWriter,
                "Total time consumed: {} sec ({:.2}%)", total_time, total_time * 100.0 / seconds_elapsed
            );
        }
        let _ = writeln!(KernelWriter);
    }

    let _ = writeln!(KernelWriter, "Total time elapsed: {} sec", seconds_elapsed);
}

// FIXME: Remove this debugging aid.
struct KernelWriter;

impl core::fmt::Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        unsafe {
            asm!(
                "svc 0xff00",
                in("x2") u64::from(u32::from(c)),
                options(nomem, preserves_flags, nostack)
            );
        }
        Ok(())
    }
}

#[panic_handler]
fn panic_handler(p: &core::panic::PanicInfo) -> ! {
    let _ = write!(KernelWriter, "Unexpected error: {}\n", p);
    syscall::thread_exit(255) // TODO: Use a named constant for the exit status.
}

#[repr(u32)]
enum QueueIndex {
    Control = 0,
    Cursor  = 1,
    Count   = 2
}

struct ConfigurationSpace<'a> {
    regs:   &'a mut [u32],
    legacy: bool
}

#[allow(dead_code)]
impl<'a> ConfigurationSpace<'a> {
    // The number of bytes expected to be in the device's configuration space (i.e. referenced by `regs`).
    const SIZE: usize = 3 * mem::size_of::<u32>();

    fn new(device_details: &'a mut DeviceDetails) -> Self {
        let legacy = device_details.legacy();
        let byte_slice = device_details.configuration_space();
        let regs = unsafe {
            assert_eq!(
                byte_slice as *mut [u8] as *mut u8 as usize % mem::align_of::<u32>(),
                0,
                "configuration space is misaligned"
            );
            slice::from_raw_parts_mut(
                byte_slice as *mut [u8] as *mut u32,
                byte_slice.len() * mem::size_of::<u8>() / mem::size_of::<u32>()
            )
        };
        Self { regs, legacy }
    }

    fn events(&mut self) -> u32 {
        unsafe { u32::from_device_endian((&self.regs[0] as *const u32).read_volatile(), self.legacy) }
    }

    fn clear_events(&mut self, events: u32) -> &Self {
        unsafe { (&mut self.regs[1] as *mut u32).write_volatile(events.to_device_endian(self.legacy)); }
        self
    }

    fn num_scanouts(&mut self) -> u32 {
        unsafe { u32::from_device_endian((&self.regs[2] as *const u32).read_volatile(), self.legacy) }
    }
}

bitflags! {
    struct Features: u64 {
        // GPU-specific
        const GPU_VIRGL = 0x0000_0000_0000_0001;
        const GPU_EDID  = 0x0000_0000_0000_0002;

        // Generic
        const NOTIFY_ON_EMPTY     = GenericFeatures::NOTIFY_ON_EMPTY.bits();
        const ANY_LAYOUT          = GenericFeatures::ANY_LAYOUT.bits();
        const RING_INDIRECT_DESC  = GenericFeatures::RING_INDIRECT_DESC.bits();
        const RING_EVENT_INDEX    = GenericFeatures::RING_EVENT_INDEX.bits();
        const VERSION_1           = GenericFeatures::VERSION_1.bits();
        const ACCESS_PLATFORM     = GenericFeatures::ACCESS_PLATFORM.bits();
        const RING_PACKED         = GenericFeatures::RING_PACKED.bits();
        const IN_ORDER            = GenericFeatures::IN_ORDER.bits();
        const ORDER_PLATFORM      = GenericFeatures::ORDER_PLATFORM.bits();
        const SINGLE_ROOT_IO_VIRT = GenericFeatures::SINGLE_ROOT_IO_VIRT.bits();
        const NOTIFICATION_DATA   = GenericFeatures::NOTIFICATION_DATA.bits();
    }
}
