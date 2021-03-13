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

//! TODO: This is just a prototype of a 2D graphics driver, designed to support both pixel-granular
//! framebuffers and tile-based framebuffers. It will need to be separated from the kernel.
//! The key is to see a pixel-based system as a special case of a tile-based system, in which each
//! tile is a 1-by-1-pixel rectangle. Likewise, a text-based system is just a tile-based system in
//! which the tiles are assigned their graphical representations from a font.

// TODO: For each `println!` in this file, consider whether it should be in the final version. Most
//       likely, the ones that stay should be changed to a call to some logging function instead.

use {
    alloc::{
        alloc::{alloc, AllocError, Layout},
        boxed::Box,
        vec::Vec
    },
    core::{
        cell::RefCell,
        convert::{TryInto, TryFrom},
        fmt::{self, Debug},
        future::Future,
        iter::{self, Rev},
        mem,
        num::{NonZeroU32, NonZeroUsize},
        ops::{Deref, Range},
        pin::Pin,
        ptr,
        slice,
        sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering},
        task::{Context, Poll, Waker, RawWaker, RawWakerVTable}
    },
    spin::Mutex,
    volatile::{Volatile, ReadOnly, WriteOnly},
    shared::{
        once::Once,
        ffi::{Le, Endian, PrimitiveEndian}
    },
    io::println,
    memory::{
        allocator::AllMemAlloc,
        phys::{
            block::{Block, BlockMut, Mmio},
            ptr::PhysPtr
        }
    }
};

// ************************
// Generic VirtIO Operation
// ************************

pub struct Device {
    pub base_addr: Address
}

pub enum Address {
    /// A physical-memory-mapped address
    Mmio(usize),
    // TODO: Pci,
    // TODO: ChannelIO (only for S/390-based virtual machines, so I probably don't need this)
}

enum Registers {
    Mmio(Mmio<MmioRegisters>)
}

static REGISTERS: Once<Registers> = Once::new();
static VERSION_1: Once<bool>      = Once::new();
static CONTROL_Q: Once<VirtQueue> = Once::new();
static CURSOR_Q:  Once<VirtQueue> = Once::new();

/// This function emulates the entry point of the graphics driver.
pub fn main(device: *const Device) {
    match unsafe { device.as_ref() } {
        None => return, // We can't do anything with a null pointer!
        Some(device) => {
            match device.base_addr {
                Address::Mmio(phys_base) => {
                    match AllMemAlloc.mmio_mut(phys_base, mem::size_of::<MmioRegisters>()) {
                        Err(AllocError) => return,
                        Ok(mmio) => {
                            if !validate_mmio(&mmio) {
                                return;
                            }
                            unsafe { REGISTERS.call_once(move || Registers::Mmio(mmio)); }
                        }
                    }
                }
            }
        }
    };

    match REGISTERS.try_get() {
        Some(Registers::Mmio(mmio)) => {
            let regs = unsafe { &mut *mmio.index(0) };
            regs.status.write(DeviceStatus::empty()); // Reset the device.
            regs.status.write(regs.status.read() | DeviceStatus::ACKNOWLEDGE); // TODO: Do these writes need to be atomic?
            regs.status.write(regs.status.read() | DeviceStatus::DRIVER);

            let (features_low, features_high);
            unsafe {
                regs.device_features_select.write(Le::<u32>::from_native(0));
                features_low = regs.device_features.read().low &
                    FeaturesLow::ANY_LAYOUT;
                regs.device_features_select.write(Le::<u32>::from_native(1));
                features_high = regs.device_features.read().high &
                    (FeaturesHigh::VERSION_1 /* TODO: | FeaturesHigh::RING_PACKED */ /* TODO: | FeaturesHigh::IN_ORDER */
                        | FeaturesHigh::ORDER_PLATFORM | FeaturesHigh::NOTIFICATION_DATA);
            }

            regs.driver_features_select.write(Le::<u32>::from_native(0));
            regs.driver_features.write(Features { low: features_low });
            regs.driver_features_select.write(Le::<u32>::from_native(1));
            regs.driver_features.write(Features { high: features_high });
            let version_1 = features_high.contains(FeaturesHigh::VERSION_1);
            unsafe { VERSION_1.call_once(|| version_1); }
            if version_1 { // "Legacy" devices didn't have the FEATURES_OK bit.
                regs.status.write(regs.status.read() | DeviceStatus::FEATURES_OK);

                // Make sure the device supports all of the features we requested.
                if !regs.status.read().contains(DeviceStatus::FEATURES_OK) {
                    regs.status.write(regs.status.read() | DeviceStatus::FAILED);
                    return;
                }
            } else {
                regs.guest_page_size.write(Le::<u32>::from_native(VirtQueue::PAGE_SIZE.try_into().unwrap()));

                println!("Legacy GPU device");
            }

            // Initialize the control queue.
            regs.queue_select.write(QueueSelect::Control);
            if version_1 {
                assert_eq!(regs.queue_ready.read().into_native(), 0, "virtio GPU: control queue already in use");
            } else {
                assert_eq!(regs.queue_page_number.read().into_native(), 0, "virtio GPU: control queue already in use");
            }
            let max_control_q_len = regs.queue_len_max.read().into_native();
            println!("Max control queue length = {}", max_control_q_len);
            if max_control_q_len == 0 {
                // This virtqueue is not available, and we can't proceed without it.
                regs.status.write(regs.status.read() | DeviceStatus::FAILED);
                return;
            }
            let control_q_len = u32::min(max_control_q_len, 0x1000);
            let control_q = VirtQueue::new(QueueSelect::Control, control_q_len as u16, VirtqDriverFlags::NO_INTERRUPT);
            regs.queue_len.write(Le::<u32>::from_native(control_q_len));
            if version_1 {
                let descriptors_addr_phys = control_q.descriptors_addr_phys();
                regs.queue_desc_low.write(Le::<u32>::from_native(descriptors_addr_phys as u32));
                regs.queue_desc_high.write(Le::<u32>::from_native((descriptors_addr_phys >> 32) as u32));
                let driver_addr_phys = control_q.driver_addr_phys();
                regs.queue_driver_low.write(Le::<u32>::from_native(driver_addr_phys as u32));
                regs.queue_driver_high.write(Le::<u32>::from_native((driver_addr_phys >> 32) as u32));
                let device_addr_phys = control_q.device_addr_phys();
                regs.queue_device_low.write(Le::<u32>::from_native(device_addr_phys as u32));
                regs.queue_device_high.write(Le::<u32>::from_native((device_addr_phys >> 32) as u32));
                regs.queue_ready.write(Le::<u32>::from_native(1));
            } else {
                regs.device_ring_align.write(Le::<u32>::from_native(VirtQueue::DEVICE_RING_ALIGN.try_into().unwrap()));
                let page_number = (control_q.descriptors_addr_phys() / VirtQueue::PAGE_SIZE).try_into()
                    .expect("virtio GPU: control virtqueue address is too high");
                regs.queue_page_number.write(Le::<u32>::from_native(page_number));
                println!("Control queue page number: {:#x}", page_number);
            }
            unsafe { CONTROL_Q.call_once(move || control_q); }

            // Initialize the cursor queue.
            regs.queue_select.write(QueueSelect::Cursor);
            if version_1 {
                assert_eq!(regs.queue_ready.read().into_native(), 0, "virtio GPU: cursor queue already in use");
            } else {
                assert_eq!(regs.queue_page_number.read().into_native(), 0, "virtio GPU: cursor queue already in use");
            }
            let max_cursor_q_len = regs.queue_len_max.read().into_native();
            println!("Max cursor queue length = {}", max_cursor_q_len);
            if max_control_q_len > 0 {
                let cursor_q_len = u32::min(max_cursor_q_len, 0x1000);
                let cursor_q = VirtQueue::new(QueueSelect::Cursor, cursor_q_len as u16, VirtqDriverFlags::NO_INTERRUPT);
                regs.queue_len.write(Le::<u32>::from_native(cursor_q_len));
                if version_1 {
                    let descriptors_addr_phys = cursor_q.descriptors_addr_phys();
                    regs.queue_desc_low.write(Le::<u32>::from_native(descriptors_addr_phys as u32));
                    regs.queue_desc_high.write(Le::<u32>::from_native((descriptors_addr_phys >> 32) as u32));
                    let driver_addr_phys = cursor_q.driver_addr_phys();
                    regs.queue_driver_low.write(Le::<u32>::from_native(driver_addr_phys as u32));
                    regs.queue_driver_high.write(Le::<u32>::from_native((driver_addr_phys >> 32) as u32));
                    let device_addr_phys = cursor_q.device_addr_phys();
                    regs.queue_device_low.write(Le::<u32>::from_native(device_addr_phys as u32));
                    regs.queue_device_high.write(Le::<u32>::from_native((device_addr_phys >> 32) as u32));
                    regs.queue_ready.write(Le::<u32>::from_native(1));
                    unsafe { CURSOR_Q.call_once(move || cursor_q); }
                } else {
                    regs.device_ring_align.write(Le::<u32>::from_native(VirtQueue::DEVICE_RING_ALIGN.try_into().unwrap()));
                    let page_number = (cursor_q.descriptors_addr_phys() / VirtQueue::PAGE_SIZE).try_into()
                        .expect("virtio GPU: cursor virtqueue address is too high");
                    regs.queue_page_number.write(Le::<u32>::from_native(page_number));
                    println!("Cursor queue page number: {:#x}", page_number);
                }
            }

            regs.status.write(regs.status.read() | DeviceStatus::DRIVER_OK);

            #[cfg(all(feature = "self-test", not(feature = "unit-test")))]
            println!("Virtio GPU driver: Setup done. Device status = {:#x}, features = {:#x}_{:08x}", regs.status.read(), features_high, features_low);

            // The driver is set up, so try out some drawing routines.
            static FRAMEBUFFER: Mutex<Option<Image>> = Mutex::new(None);

            EXECUTOR.spawn(async {
                let disp_info = match DisplayInfo::one(0).await {
                    Ok(info) => info,
                    Err(()) => panic!("Failed to retrieve display info")
                };
                assert!(disp_info.flags.contains(DisplayInfoFlags::ENABLED));

                let fb = Image::new(true, 0, 1, 32, disp_info.rect.width, disp_info.rect.height).await
                    .expect("failed to create framebuffer");
                set_display_framebuffer(0, &fb).await
                    .expect("failed to set display framebuffer");
                *FRAMEBUFFER.lock() = Some(fb);
            }).execute_blocking();

            let mut locked_fb = FRAMEBUFFER.lock();
            let fb = locked_fb.as_mut().unwrap();
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
            EXECUTOR.spawn(async move {
                let fb = locked_fb.as_ref().unwrap();
                fb.send_all().await
                    .expect("failed to send the framebuffer to the host");
                fb.flush_all().await
                    .expect("failed to flush the framebuffer");
            }).execute_blocking();
        },
        None => panic!("virtio GPU registers haven't been found")
    };
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
struct DeviceEndian<T: PrimitiveEndian+fmt::Debug+Clone+Copy+PartialEq+Eq>(T);

impl<T: PrimitiveEndian+fmt::Debug+Clone+Copy+PartialEq+Eq> Endian for DeviceEndian<T> {
    type Primitive = T;

    fn from_native(val: T) -> Self {
        if *VERSION_1.try_get().unwrap() {
            Self(val.to_le())
        } else {
            Self(val)
        }
    }

    fn into_native(self) -> T {
        if *VERSION_1.try_get().unwrap() {
            T::from_le(self.0)
        } else {
            self.0
        }
    }
}

impl<T: PrimitiveEndian+fmt::Debug+Clone+Copy+PartialEq+Eq> fmt::Debug for DeviceEndian<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("DeviceEndian")
            .field(&self.0)
            .finish()?;
        write!(f, " {{ native endian = ")?;
        fmt::Debug::fmt(&self.into_native(), f)?;
        write!(f, " }}")
    }
}

// Makes sure this driver can actually handle the given device.
fn validate_mmio(mmio: &Mmio<MmioRegisters>) -> bool {
    const MAGIC_NUMBER:    u32 = 0x74726976; // Little-endian "virt"
    const CURRENT_VERSION: u32 = 1; // The latest version is 2, but QEMU's implementation is still version 1.
    const DEVICE_TYPE_GPU: u32 = 16;

    let regs = unsafe { &*mmio.index(0) };

    if regs.magic_number.read().into_native() != MAGIC_NUMBER {
        return false; // Not a VirtIO device
    }
    let version = regs.version.read().into_native();
    if version < 1 || version > CURRENT_VERSION {
        return false; // Not a version we support
    }
    if regs.device_id.read().into_native() != DEVICE_TYPE_GPU {
        return false; // Not a GPU
    }
    if regs.num_scanouts.read() == DeviceEndian(0) {
        return false; // No scanouts (i.e. monitors)
    }

    true
}

/// This struct defines the layout of all of the registers that the device provides when its
/// discoverability isn't handled by something like a PCI bus.
#[repr(C)]
struct MmioRegisters {
    magic_number:           ReadOnly<Le<u32>>,
    version:                ReadOnly<Le<u32>>,
    device_id:              ReadOnly<Le<u32>>,
    vendor_id:              ReadOnly<Le<u32>>,
    device_features:        ReadOnly<Features>,
    device_features_select: WriteOnly<Le<u32>>,
    padding1:               ReadOnly<[u32; 2]>,
    driver_features:        WriteOnly<Features>,
    driver_features_select: WriteOnly<Le<u32>>,
    guest_page_size:        WriteOnly<Le<u32>>,     // Only used by legacy devices
    padding2:               ReadOnly<[u32; 1]>,
    queue_select:           WriteOnly<QueueSelect>,
    queue_len_max:          ReadOnly<Le<u32>>,
    queue_len:              WriteOnly<Le<u32>>,
    device_ring_align:      WriteOnly<Le<u32>>,     // Only used by legacy devices
    queue_page_number:      Volatile<Le<u32>>,      // Only used by legacy devices
    queue_ready:            Volatile<Le<u32>>,
    padding3:               ReadOnly<[u32; 2]>,
    queue_notify:           WriteOnly<QueueSelect>,
    padding4:               ReadOnly<[u32; 3]>,
    interrupt_status:       ReadOnly<Le<u32>>,
    interrupt_ack:          WriteOnly<Le<u32>>,
    padding5:               ReadOnly<[u32; 2]>,
    status:                 Volatile<DeviceStatus>,
    padding6:               ReadOnly<[u32; 3]>,
    queue_desc_low:         WriteOnly<Le<u32>>,
    queue_desc_high:        WriteOnly<Le<u32>>,
    padding7:               ReadOnly<[u32; 2]>,
    queue_driver_low:       WriteOnly<Le<u32>>,
    queue_driver_high:      WriteOnly<Le<u32>>,
    padding8:               ReadOnly<[u32; 2]>,
    queue_device_low:       WriteOnly<Le<u32>>,
    queue_device_high:      WriteOnly<Le<u32>>,
    padding9:               ReadOnly<[u32; 21]>,
    config_generation:      ReadOnly<Le<u32>>,

    // GPU-specific device configuration registers
    events:                 ReadOnly<DeviceEndian<u32>>,
    events_clear:           WriteOnly<DeviceEndian<u32>>,
    num_scanouts:           ReadOnly<DeviceEndian<u32>>
}

bitflags! {
    struct FeaturesLow: u32 {
        // GPU-specific
        const GPU_VIRGL          = u32::to_le(0x0000_0001);
        const GPU_EDID           = u32::to_le(0x0000_0002);

        // Generic
        const NOTIFY_ON_EMPTY    = u32::to_le(0x0100_0000);
        const ANY_LAYOUT         = u32::to_le(0x0800_0000);
        const RING_INDIRECT_DESC = u32::to_le(0x1000_0000);
        const RING_EVENT_INDEX   = u32::to_le(0x2000_0000);
    }
}

bitflags! {
    struct FeaturesHigh: u32 {
        const VERSION_1           = u32::to_le(0x0000_0001);
        const ACCESS_PLATFORM     = u32::to_le(0x0000_0002);
        const RING_PACKED         = u32::to_le(0x0000_0004);
        const IN_ORDER            = u32::to_le(0x0000_0008);
        const ORDER_PLATFORM      = u32::to_le(0x0000_0010);
        const SINGLE_ROOT_IO_VIRT = u32::to_le(0x0000_0020);
        const NOTIFICATION_DATA   = u32::to_le(0x0000_0040);
    }
}

#[derive(Clone, Copy)]
union Features {
    low:  FeaturesLow,
    high: FeaturesHigh,
    raw:  u32
}

impl PartialEq for Features {
    fn eq(&self, other: &Features) -> bool {
        unsafe { self.raw == other.raw }
    }
}
impl Eq for Features {}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueueSelect {
    Control = u32::to_le(0),
    Cursor  = u32::to_le(1)
}

bitflags! {
    struct DeviceStatus: u32 {
        const ACKNOWLEDGE = u32::to_le(0x01); // OS has noticed the device
        const DRIVER      = u32::to_le(0x02); // OS knows how to drive the device
        const DRIVER_OK   = u32::to_le(0x04); // Driver is ready
        const FEATURES_OK = u32::to_le(0x08); // Driver has acknowledged the features it understands
        const NEEDS_RESET = u32::to_le(0x40); // Device has experienced an error and needs to be reset
        const FAILED      = u32::to_le(0x80); // OS has given up on the device
    }
}

struct VirtQueue {
    id: QueueSelect,
    descriptors: BlockMut<VirtqBufferDescriptor>,
    descriptors_len: usize,
    free_descs: AtomicU16,
    first_free_desc_idx: AtomicU16,
    driver_ring: VirtqDriverRing,
    device_ring: VirtqDeviceRing,
    last_dev_ring_idx: AtomicU16, // TODO: This should probably be in the device ring enum, not here.
    wakers: Box<[RefCell<Option<Waker>>]>
}

impl fmt::Debug for VirtQueue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VirtQueue")
            .field("id", &self.id)
            .field("descriptors", &unsafe { slice::from_raw_parts(self.descriptors.index(0), self.descriptors_len) })
            .field("free_descs", &self.free_descs)
            .field("first_free_desc_idx", &self.first_free_desc_idx)
            .field("driver_ring", &self.driver_ring)
            .field("device_ring", &self.device_ring)
            .field("last_dev_ring_idx", &self.last_dev_ring_idx)
            .field("wakers", &self.wakers)
            .finish()
    }
}

impl VirtQueue {
    // These are only needed for legacy devices.
    const PAGE_SIZE:         usize = 0x10000;
    const DEVICE_RING_ALIGN: usize = 0x1000;

    fn new(id: QueueSelect, len: u16, driver_flags: VirtqDriverFlags) -> VirtQueue {
        let len = len as usize;
        let mut desc_is_free = Vec::with_capacity(len);
        desc_is_free.resize_with(len, || AtomicBool::new(true));

        let descriptors;
        let driver_ring;
        let device_ring;
        if *VERSION_1.try_get().unwrap() {
            descriptors =
                AllMemAlloc.malloc(mem::size_of::<VirtqBufferDescriptor>() * len, NonZeroUsize::new(mem::align_of::<VirtqBufferDescriptor>()).unwrap())
                    .expect("not enough memory for a new virtqueue");
            driver_ring = unsafe {
                VirtqDriverRing::from_block(
                    AllMemAlloc.malloc(mem::size_of::<u16>() * (3 + len), NonZeroUsize::new(mem::align_of::<u16>()).unwrap())
                        .expect("not enough memory for a new virtqueue")
                        .into(),
                    driver_flags
                )
            };
            device_ring = unsafe {
                VirtqDeviceRing::from_block(
                    AllMemAlloc.malloc(mem::size_of::<u16>() * 3 + mem::size_of::<VirtqDeviceDescriptor>() * len, NonZeroUsize::new(4).unwrap())
                        .expect("not enough memory for a new virtqueue")
                        .into()
                )
            };
        } else {
            // In "legacy" devices, everything needs to be roughly contiguous.
            const ALIGN: usize = VirtQueue::DEVICE_RING_ALIGN;
            descriptors = AllMemAlloc.malloc(
                ((mem::size_of::<VirtqBufferDescriptor>() * len + mem::size_of::<u16>() * (3 + len) + ALIGN) & !(ALIGN - 1)) +
                ((mem::size_of::<u16>() * 3 + mem::size_of::<VirtqDeviceDescriptor>() * len + ALIGN) & !(ALIGN - 1)),
                NonZeroUsize::new(usize::max(ALIGN, Self::PAGE_SIZE)).unwrap()
            ).expect("not enough memory for a new virtqueue");
            let base = descriptors.base().as_addr_phys();
            let driver_base = base + mem::size_of::<VirtqBufferDescriptor>() * len;
            driver_ring = unsafe { VirtqDriverRing::from_ptr(PhysPtr::<_, *const _>::from_addr_phys(driver_base), len, driver_flags) };
            let device_base = (driver_base + mem::size_of::<u16>() * (3 + len) + ALIGN) & !(ALIGN - 1);
            device_ring = unsafe { VirtqDeviceRing::from_ptr(PhysPtr::<_, *const _>::from_addr_phys(device_base), len) };
        }

        let mut wakers = Vec::with_capacity(len);
        wakers.resize_with(len, || RefCell::new(None));
        let wakers = wakers.into_boxed_slice();

        for i in 0 .. len {
            unsafe {
                mem::forget(mem::replace(&mut *descriptors.index(i), VirtqBufferDescriptor::new(0, 0, VirtqBufferFlags::empty(), ((i + 1) % len) as u16)));
            }
        }

        VirtQueue {
            id,
            descriptors,
            descriptors_len: len,
            free_descs: AtomicU16::new(len as u16),
            first_free_desc_idx: AtomicU16::new(0),
            driver_ring,
            device_ring,
            last_dev_ring_idx: AtomicU16::new(0),
            wakers
        }
    }

    /// Asynchronously sends a message to the device and returns its response.
    ///
    /// # Returns
    /// * `Ok(Ok)` if the send succeeds (the receive might still fail, depending on the device)
    /// * `Ok(Err(buf))` if the send fails because the virtqueue doesn't have enough free descriptors
    /// * `Err` if the send fails for any other reason
    // TODO: This function really needs to be refactored.
    // TODO: The return type is confusing. We should make a new `enum` that's equivalent to a `Result` but doesn't imply an error.
    fn send_recv<T: ?Sized+Debug>(&self, buf: Box<T>, first_recv_idx: usize, legacy_response_len: usize)
            -> io::Result<Result<FutureResponse<Box<T>>, Box<T>>> {
        let buf_size = mem::size_of_val(&*buf);

        if buf_size > u32::max_value() as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, VirtIoError::new("attempted to write a buffer of at least 4 GiB to a VirtIO GPU")));
        }

        // If the buffer is empty, we don't actually need to do anything.
        if buf_size == 0 {
            return Ok(Ok(FutureResponse::new_immediate(buf)));
        }

        // We need one descriptor for output and one for input.
        // If `first_recv_idx` is past the end of `buf`, we're only outputting.
        // If it's 0, we're only inputting.
        let descriptors_needed = if first_recv_idx >= buf_size || first_recv_idx == 0 { 1 } else { 2 };

        // Decrease the number of free descriptors by the number needed. If we can do that without
        // underflowing, we're guaranteed to find enough that are available.
        let mut free_descs = self.free_descs.load(Ordering::Acquire);
        loop {
            if free_descs < descriptors_needed {
                return Ok(Err(buf)); // Can't write right now. Try again later.
            }
            match self.free_descs.compare_exchange(free_descs, free_descs - descriptors_needed, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break,
                Err(x) => free_descs = x
            };
        }

        // TODO: These two if-else blocks are nearly identical. Refactor them into another function.
        // Find and use the next free descriptor for output.
        let send_idx = if first_recv_idx > 0 {
            unsafe {
                let next = &self.first_free_desc_idx;
                let mut idx = DeviceEndian(next.load(Ordering::Acquire));
                loop {
                    match next.compare_exchange(
                        idx.0,
                        (*self.descriptors.index(idx.into_native() as usize)).next.load(Ordering::Acquire),
                        Ordering::SeqCst,
                        Ordering::Acquire
                    ) {
                        Ok(_) => break,
                        Err(x) => idx = DeviceEndian(x)
                    };
                }
                let idx = idx.into_native() as usize;

                // Give the pointer and length of the output buffer to the device.
                let desc = &mut *self.descriptors.index(idx);
                let phys_ptr = PhysPtr::<u8, *const u8>::from(&*buf as *const _ as *const u8);
                let len = if first_recv_idx < buf_size { first_recv_idx } else { buf_size };
                desc.addr.write(DeviceEndian::from_native(phys_ptr.as_addr_phys().try_into().unwrap()));
                desc.len.write(DeviceEndian::from_native(len.try_into().unwrap()));
                desc.flags.write(DeviceEndian::from_native(
                    if first_recv_idx < buf_size { VirtqBufferFlags::NEXT } else { VirtqBufferFlags::empty() }.bits()
                ));

                Some(idx as u16)
            }
        } else {
            None
        };

        // Find and use the next free descriptor for input.
        let recv_idx = if first_recv_idx < buf_size {
            unsafe {
                let next = &self.first_free_desc_idx;
                let mut idx = DeviceEndian(next.load(Ordering::Acquire));
                loop {
                    match next.compare_exchange(
                        idx.0,
                        (*self.descriptors.index(idx.into_native() as usize)).next.load(Ordering::Acquire),
                        Ordering::SeqCst,
                        Ordering::Acquire
                    ) {
                        Ok(_) => break,
                        Err(x) => idx = DeviceEndian(x)
                    };
                }
                let idx = idx.into_native() as usize;

                // Give the pointer and length of the input buffer to the device.
                let desc = &mut *self.descriptors.index(idx);
                let phys_ptr = PhysPtr::<u8, *const u8>::from((&*buf as *const _ as *const u8).add(first_recv_idx));
                let len = buf_size - first_recv_idx;
                desc.addr.write(DeviceEndian::from_native(phys_ptr.as_addr_phys().try_into().unwrap()));
                desc.len.write(DeviceEndian::from_native(len.try_into().unwrap()));
                desc.flags.write(DeviceEndian::from_native(VirtqBufferFlags::WRITE.bits()));

                Some(idx as u16)
            }
        } else {
            None
        };

        // Give the device the index of the first descriptor in the chain.
        let head_idx = match (send_idx, recv_idx) {
            (Some(idx), _)    => idx,
            (None, Some(idx)) => idx,
            (None, None)      => unreachable!()
        };
        self.driver_ring.set_next_entry(head_idx);

        // Notify the device of the new buffers if it expects notifications.
        if !self.device_ring.flags().contains(VirtqDeviceFlags::NO_INTERRUPT) {
            match REGISTERS.try_get() {
                Some(Registers::Mmio(ref mmio)) => {
                    let regs = unsafe { &mut *mmio.index(0) };
                    regs.queue_notify.write(self.id);
                },
                None => panic!("virtio GPU registers haven't been found")
            }
        }

        // Wait for the device to respond.
        let tail_idx = match (send_idx, recv_idx) {
            (_, Some(idx))    => idx,
            (Some(idx), None) => idx,
            (None, None)      => unreachable!()
        };
        Ok(Ok(FutureResponse::new(self, head_idx, tail_idx, descriptors_needed, buf, legacy_response_len)))
    }

    fn descriptors_addr_phys(&self) -> usize {
        self.descriptors.base().as_addr_phys()
    }

    fn driver_addr_phys(&self) -> usize {
        match self.driver_ring {
            VirtqDriverRing::Block { ref block, .. } => block.base().as_addr_phys(),
            VirtqDriverRing::Ptr { ref ptr, .. } => ptr.as_addr_phys()
        }
    }

    fn device_addr_phys(&self) -> usize {
        match self.device_ring {
            VirtqDeviceRing::Block(ref block) => block.base().as_addr_phys(),
            VirtqDeviceRing::Ptr(ref ptr, _) => ptr.as_addr_phys()
        }
    }
}

unsafe impl<'a> Sync for VirtQueue {}

enum VirtqDriverRing {
    Block {
        block: Block<AtomicU16>,
        state: VirtqDriverRingState
    },
    Ptr {
        ptr: PhysPtr<AtomicU16, *const AtomicU16>,
        len: usize,
        state: VirtqDriverRingState
    }
}

#[derive(Debug)]
struct VirtqDriverRingState {
    next_idx: AtomicU16,
    entries_updated: Vec<AtomicBool>
}

bitflags! {
    struct VirtqDriverFlags: u16 {
        const NO_INTERRUPT = 0x0001;
    }
}

impl VirtqDriverRing {
    const FLAGS_OFFSET: usize = 0;
    const IDX_OFFSET:   usize = 1;
    const RING_OFFSET:  usize = 2;

    unsafe fn from_block(block: Block<AtomicU16>, flags: VirtqDriverFlags) -> VirtqDriverRing {
        (*block.index(Self::FLAGS_OFFSET)).store(DeviceEndian::from_native(flags.bits()).0, Ordering::Release);
        (*block.index(Self::IDX_OFFSET)).store(DeviceEndian::from_native(0).0, Ordering::Release);
        let len = block.size() - 3; // block.size() = number of entries + flags, idx, and used_event
        VirtqDriverRing::Block {
            block,
            state: VirtqDriverRingState {
                next_idx: AtomicU16::new(0),
                entries_updated: iter::repeat(())
                    .take(len)
                    .map(|()| AtomicBool::new(false))
                    .collect()
            }
        }
    }

    unsafe fn from_ptr(ptr: PhysPtr<AtomicU16, *const AtomicU16>,
            len: usize, flags: VirtqDriverFlags) -> VirtqDriverRing {
        (*ptr.add(Self::FLAGS_OFFSET).as_virt_unchecked()).store(DeviceEndian::from_native(flags.bits()).0, Ordering::Release);
        (*ptr.add(Self::IDX_OFFSET).as_virt_unchecked()).store(DeviceEndian::from_native(0).0, Ordering::Release);
        VirtqDriverRing::Ptr {
            ptr,
            len,
            state: VirtqDriverRingState {
                next_idx: AtomicU16::new(0),
                entries_updated: iter::repeat(())
                    .take(len)
                    .map(|()| AtomicBool::new(false))
                    .collect()
            }
        }
    }

    fn flags(&self) -> u16 {
        let idx_de = unsafe {
            match *self {
                VirtqDriverRing::Block { ref block, .. } => &*block.index(Self::FLAGS_OFFSET),
                VirtqDriverRing::Ptr { ref ptr, .. } => &*ptr.add(Self::FLAGS_OFFSET).as_virt_unchecked()
            }
        }.load(Ordering::Acquire);
        DeviceEndian(idx_de).into_native()
    }

    fn idx(&self) -> u16 {
        let idx_de = unsafe {
            match *self {
                VirtqDriverRing::Block { ref block, .. } => &*block.index(Self::IDX_OFFSET),
                VirtqDriverRing::Ptr { ref ptr, .. } => &*ptr.add(Self::IDX_OFFSET).as_virt_unchecked()
            }
        }.load(Ordering::Acquire);
        DeviceEndian(idx_de).into_native()
    }

    fn add_idx(&self, steps: u16) {
        let idx_de = unsafe {
            match *self {
                VirtqDriverRing::Block { ref block, .. } => &*block.index(Self::IDX_OFFSET),
                VirtqDriverRing::Ptr { ref ptr, .. } => &*ptr.add(Self::IDX_OFFSET).as_virt_unchecked()
            }
        };
        if cfg!(not(target_endian = "little")) && *VERSION_1.try_get().unwrap() {
            // This device uses little-endian regardless of the guest CPU.
            let mut old_idx = idx_de.load(Ordering::Acquire);
            loop {
                let old_idx_be = u16::from_le(old_idx);
                match idx_de.compare_exchange_weak(old_idx, u16::to_le(old_idx_be + steps),
                        Ordering::SeqCst, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(x) => old_idx = x
                };
            }
        } else {
            // A little-endian CPU always matches the device's endianness, and a legacy device
            // always matches the guest CPU's endianness.
            idx_de.fetch_add(steps, Ordering::SeqCst);
        }
    }

    /*fn set_used_event(&self, flags: VirtqDriverFlags) {
        // TODO: This should probably use a compare_exchange. We don't need it for this driver
        // anyway. (Technically, no driver needs it, since it's only used for suppressing
        // notifications, but it's useful for optimization.)
        unsafe {
            match *self {
                VirtqDriverRing::Block(ref block) => &mut *block.index(self.used_event_offset()),
                VirtqDriverRing::Ptr(ref ptr, _) => &mut *ptr.add(self.used_event_offset()).as_virt_unchecked()
            }
        }.store(flags.bits().into())
    }*/

    fn set_next_entry(&self, val: u16) {
        let mut this_idx;
        let entries_updated;
        unsafe {
            match *self {
                VirtqDriverRing::Block { ref block, ref state } => {
                    this_idx = state.next_idx.fetch_add(1, Ordering::AcqRel);
                    entries_updated = &state.entries_updated;
                    &*block.index(Self::RING_OFFSET + this_idx as usize % self.len())
                },
                VirtqDriverRing::Ptr { ref ptr, ref state, .. } => {
                    this_idx = state.next_idx.fetch_add(1, Ordering::AcqRel);
                    entries_updated = &state.entries_updated;
                    &*ptr.add(Self::RING_OFFSET + this_idx as usize % self.len()).as_virt_unchecked()
                }
            }
        }.store(val, Ordering::SeqCst);

        // `self.idx()` must never skip over an entry that hasn't actually been updated yet. If we
        // are ahead of that device-visible index, just leave a note for the task that's not ahead
        // of it to handle our update for us. If we're not, then handle all those updates, cleaning
        // up the notes as we go.
        assert!(!entries_updated[this_idx as usize % self.len()].swap(true, Ordering::SeqCst));
        if this_idx == self.idx() {
            loop {
                let steps = entries_updated.iter()
                    .cycle() // This is a circular array. No need for `take` because we'll find a `false` by the time we revolve once.
                    .skip(this_idx as usize % self.len())
                    .take_while(|x| x.swap(false, Ordering::SeqCst))
                    .count() as u16;
                self.add_idx(steps);

                // If, between the last `swap` and `add_idx`, a new note was left at the next
                // index, we have to keep going.
                this_idx = this_idx.wrapping_add(steps);
                if !entries_updated[this_idx as usize % self.len()].load(Ordering::SeqCst) {
                    break;
                }
            }
        }
    }

    fn used_event_offset(&self) -> usize {
        Self::RING_OFFSET + self.len()
    }

    fn len(&self) -> usize {
        match *self {
            VirtqDriverRing::Block { ref block, .. } => block.size() - 3,
            VirtqDriverRing::Ptr { len, .. } => len
        }
    }
}

impl fmt::Debug for VirtqDriverRing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ring, state): (&[AtomicU16], &VirtqDriverRingState) = match *self {
            VirtqDriverRing::Block { ref block, ref state } => unsafe {
                (
                    slice::from_raw_parts(block.index(Self::RING_OFFSET), self.len()),
                    state
                )
            },
            VirtqDriverRing::Ptr { ptr, len: _, ref state } => unsafe {
                (
                    slice::from_raw_parts(ptr.add(Self::RING_OFFSET).as_virt().unwrap(), self.len()),
                    state
                )
            }
        };
        f.debug_struct("VirtqDriverRing")
            .field("flags", &self.flags())
            .field("idx", &self.idx())
            .field("ring", &ring)
            .field("state", state)
            .finish()
    }
}

enum VirtqDeviceRing {
    Block(Block<AtomicU16>),
    Ptr(PhysPtr<AtomicU16, *const AtomicU16>, usize)
}

bitflags! {
    struct VirtqDeviceFlags: u16 {
        const NO_INTERRUPT = 0x0001;
    }
}

impl VirtqDeviceRing {
    const FLAGS_OFFSET: usize = 0;
    const IDX_OFFSET:   usize = 1;
    const RING_OFFSET:  usize = 2;

    unsafe fn from_block(block: Block<AtomicU16>) -> VirtqDeviceRing {
        (*block.index(Self::FLAGS_OFFSET)).store(0, Ordering::Release);
        (*block.index(Self::IDX_OFFSET)).store(0, Ordering::Release);
        VirtqDeviceRing::Block(block)
    }

    unsafe fn from_ptr(ptr: PhysPtr<AtomicU16, *const AtomicU16>, len: usize) -> VirtqDeviceRing {
        (*ptr.add(Self::FLAGS_OFFSET).as_virt_unchecked()).store(0, Ordering::Release);
        (*ptr.add(Self::IDX_OFFSET).as_virt_unchecked()).store(0, Ordering::Release);
        VirtqDeviceRing::Ptr(ptr, len)
    }

    fn flags(&self) -> VirtqDeviceFlags {
        VirtqDeviceFlags::from_bits_truncate(
            DeviceEndian(
                match *self {
                    VirtqDeviceRing::Block(ref block) => {
                        unsafe { (*block.index(Self::FLAGS_OFFSET)).load(Ordering::Acquire) }
                    },
                    VirtqDeviceRing::Ptr(ref ptr, _) => {
                        unsafe { (*ptr.add(Self::FLAGS_OFFSET).as_virt_unchecked()).load(Ordering::Acquire) }
                    }
                }
            ).into_native()
        )
    }

    fn idx(&self) -> u16 {
        DeviceEndian(
            match *self {
                VirtqDeviceRing::Block(ref block) => {
                    unsafe { (*block.index(Self::IDX_OFFSET)).load(Ordering::Acquire) }
                },
                VirtqDeviceRing::Ptr(ref ptr, _) => {
                    unsafe { (*ptr.add(Self::IDX_OFFSET).as_virt_unchecked()).load(Ordering::Acquire) }
                }
            }
        ).into_native()
    }

    fn ring(&self) -> &[VirtqUsedElem] {
        let ptr = match *self {
            VirtqDeviceRing::Block(ref block) => block.index(Self::RING_OFFSET),
            VirtqDeviceRing::Ptr(ref ptr, _) => unsafe { ptr.add(Self::RING_OFFSET).as_virt_unchecked() }
        } as *const VirtqUsedElem;
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }

    // The number of entries in this ring, not the size of the structure.
    fn len(&self) -> usize {
        match *self {
            VirtqDeviceRing::Block(ref block) => mem::size_of::<u16>() * (block.size() - 3) / mem::size_of::<VirtqUsedElem>(),
            VirtqDeviceRing::Ptr(_, len) => len
        }
    }
}

impl fmt::Debug for VirtqDeviceRing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ring: &[VirtqUsedElem] = match *self {
            VirtqDeviceRing::Block(ref block) => unsafe {
                slice::from_raw_parts(block.index(Self::RING_OFFSET) as *const VirtqUsedElem, self.len())
            },
            VirtqDeviceRing::Ptr(ptr, _) => unsafe {
                slice::from_raw_parts(ptr.add(Self::RING_OFFSET).as_virt().unwrap() as *const VirtqUsedElem, self.len())
            }
        };
        f.debug_struct("VirtqDeviceRing")
            .field("flags", &self.flags())
            .field("idx", &self.idx())
            .field("ring", &ring)
            .finish()
    }
}

#[derive(Debug)]
#[repr(C)]
struct VirtqUsedElem {
    id:  ReadOnly<DeviceEndian<u32>>,
    len: ReadOnly<DeviceEndian<u32>>
}

#[derive(Debug)]
#[repr(C, align(16))]
struct VirtqBufferDescriptor {
    addr:  Volatile<DeviceEndian<u64>>,
    len:   Volatile<DeviceEndian<u32>>,
    flags: Volatile<DeviceEndian<u16>>,
    next:  AtomicU16
}

bitflags! {
    struct VirtqBufferFlags: u16 {
        const NEXT     = 0x1;
        const WRITE    = 0x2;
        const INDIRECT = 0x4;
    }
}

impl VirtqBufferDescriptor {
    fn new(addr: u64, len: u32, flags: VirtqBufferFlags, next: u16) -> VirtqBufferDescriptor {
        VirtqBufferDescriptor {
            addr:  Volatile::new(DeviceEndian::from_native(addr)),
            len:   Volatile::new(DeviceEndian::from_native(len)),
            flags: Volatile::new(DeviceEndian::from_native(flags.bits())),
            next:  AtomicU16::new(DeviceEndian::from_native(next).0)
        }
    }

    fn addr(&self) -> u64 {
        self.addr.read().into_native()
    }

    fn len(&self) -> u32 {
        self.len.read().into_native()
    }

    fn flags(&self) -> VirtqBufferFlags {
        VirtqBufferFlags::from_bits(self.flags.read().into_native()).unwrap()
    }

    fn next(&self, ordering: Ordering) -> u16 {
        DeviceEndian(self.next.load(ordering)).into_native()
    }
}

#[derive(Debug)]
struct VirtqDeviceDescriptor {
    id: DeviceEndian<u32>,
    len: DeviceEndian<u32>
}

#[derive(Debug)]
struct VirtIoError {
    desc: &'static str
}

impl VirtIoError {
    fn new(desc: &'static str) -> VirtIoError {
        VirtIoError { desc }
    }
}

impl fmt::Display for VirtIoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc)
    }
}

impl error::Error for VirtIoError {}

// *******
// Futures
// *******

pub static EXECUTOR: Executor = Executor::new();

#[derive(Debug)]
struct FutureResponse<'a, P: Deref+Unpin+Debug> {
    virtq: Option<&'a VirtQueue>,
    desc_head_idx: u16,
    desc_tail_idx: u16,
    descriptors_count: u16,
    value_ptr: Option<P>,
    legacy_response_len: usize
}

impl<'a, P: Deref+Unpin+Debug> FutureResponse<'a, P> {
    /// Creates a new future that will be ready when the device sends a response for the given
    /// descriptor.
    pub const fn new(
            virtq: &'a VirtQueue,
            desc_head_idx: u16,
            desc_tail_idx: u16,
            descriptors_count: u16,
            value_ptr: P,
            legacy_response_len: usize
    ) -> FutureResponse<'a, P> {
        FutureResponse {
            virtq: Some(virtq),
            desc_head_idx,
            desc_tail_idx,
            descriptors_count,
            value_ptr: Some(value_ptr),
            legacy_response_len
        }
    }

    pub const fn new_immediate(value_ptr: P) -> FutureResponse<'a, P> {
        FutureResponse {
            virtq: None,
            desc_head_idx: 0,
            desc_tail_idx: 0,
            descriptors_count: 0,
            value_ptr: Some(value_ptr),
            legacy_response_len: 0
        }
    }
}

#[derive(Debug)]
struct Response<P: Debug> {
    value_ptr: P,
    valid_bytes: usize // The number of bytes from the beginning of *value_ptr that are defined
}

impl<'a, P: Deref+Unpin+Debug> Future for FutureResponse<'a, P> {
    type Output = Response<P>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Response<P>> {
        let desc_head_idx = self.desc_head_idx;
        let desc_tail_idx = self.desc_tail_idx;
        let descriptors_count = self.descriptors_count;

        match self.virtq {
            None => {
                // This hasn't been tied to a virtqueue, so we can't wait for a response. Just
                // return what's already there. (This is used if we try to send a zero-length
                // message.)
                let value_ptr = mem::replace(&mut self.value_ptr, None).unwrap();
                let valid_bytes = mem::size_of_val(&*value_ptr);
                Poll::Ready(Response { value_ptr, valid_bytes })
            },
            Some(ref mut virtq) => {
                // See if the device has responded yet.

                let dev_ring = virtq.device_ring.ring();
                let last_dev_ring_idx = virtq.last_dev_ring_idx.load(Ordering::Acquire);
                let dev_ring_entry = &dev_ring[last_dev_ring_idx as usize % dev_ring.len()];
                let found_desc_idx = dev_ring_entry.id.read().into_native() as u16;
                if virtq.device_ring.idx() == last_dev_ring_idx {
                    // The device hasn't read any buffers yet. Stay awake so we don't miss it.
                    // PERF: Wait for a "used buffer notification" before waking the appropriate
                    //       future to avoid needless polling.
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else if found_desc_idx == desc_head_idx {
                    /* FIXME: This breaks if the device doesn't consume all the buffers at the same time. */
                    // There is a new response at index `last_dev_ring_idx`, and it's for this future.

                    // Move to the next index, and if there's still another response, wake the
                    // appropriate future.
                    let next_idx = last_dev_ring_idx.wrapping_add(1);
                    if virtq.last_dev_ring_idx.compare_exchange(
                            last_dev_ring_idx,
                            next_idx,
                            Ordering::AcqRel,
                            Ordering::Acquire
                    ).is_ok() {
                        /* FIXME: We need to rethink how we register wakers, as this index may be wrong.
                        if let Some(waker) = virtq.wakers[desc_idx as usize].replace(None) {
                            waker.wake();
                        } */
                    }

                    // Return the descriptor chain to the list of free descriptors.
                    let mut next = DeviceEndian(virtq.first_free_desc_idx.load(Ordering::Acquire));
                    loop {
                        let desc_tail = unsafe { &*virtq.descriptors.index(desc_tail_idx as usize) };
                        desc_tail.next.store(next.0, Ordering::Release);
                        match virtq.first_free_desc_idx.compare_exchange_weak(
                                next.0,
                                desc_head_idx,
                                Ordering::AcqRel,
                                Ordering::Acquire
                        ) {
                            Ok(_) => break,
                            Err(x) => next = DeviceEndian(x) // The list has a new head. Retry with that one.
                        };
                    }
                    // Update the number of free descriptors now that they are actually available again.
                    virtq.free_descs.fetch_add(descriptors_count, Ordering::AcqRel);

                    // From the VirtIO 1.1 specification, section 2.6.8.1:
                    // "Historically, many drivers ignored the len value, as a result, many devices set len
                    // incorrectly. Thus, when using the legacy interface, it is generally a good idea to
                    // ignore the len value in used ring entries if possible."
                    // Therefore, we won't trust `len` for legacy devices.
                    let valid_bytes = if *VERSION_1.try_get().unwrap() {
                        dev_ring_entry.len.read().into_native() as usize
                    } else {
                        self.legacy_response_len
                    };

                    // Return the response.
                    Poll::Ready(Response {
                        value_ptr:   mem::replace(&mut self.value_ptr, None).unwrap(),
                        valid_bytes
                    })
                } else {
                    // The device has read at least one buffer, but it's either not yet at the end
                    // of our descriptor chain or reading a different chain entirely. Wake the future
                    // that's responsible for this descriptor chain and (if that's someone else) wait
                    // until someone wakes us up.
                    /* FIXME: We need to rethink how we register wakers, as this index may be wrong.
                    if let Some(waker) = virtq.wakers[found_desc_idx as usize].replace(None) {
                        waker.wake(); // If the waker is `None`, that future is already awake.
                    } */
                    // FIXME: *virtq.wakers[desc_idx as usize].borrow_mut() = Some(cx.waker().clone());
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
    }
}

struct FutureResponseWaker<'a> {
    executor: &'a Executor,
    future_idx: usize
}

static FUTURE_RESPONSE_RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    FutureResponseWaker::clone,
    FutureResponseWaker::wake,
    FutureResponseWaker::wake_by_ref,
    FutureResponseWaker::drop
);

impl<'a> FutureResponseWaker<'a> {
    fn new(executor: &Executor, future_idx: usize) -> FutureResponseWaker {
        FutureResponseWaker {
            executor,
            future_idx
        }
    }

    // `clone` and `drop` are defined here instead of in trait implementations because they need to
    // be used as function pointers with slightly different signatures.
    unsafe fn clone(waker: *const ()) -> RawWaker {
        let waker = waker as *const FutureResponseWaker;
        RawWaker::new(
            Box::into_raw(Box::new(FutureResponseWaker {
                executor: (*waker).executor,
                future_idx: (*waker).future_idx
            })) as *const (),
            &FUTURE_RESPONSE_RAW_WAKER_VTABLE
        )
    }

    unsafe fn wake(waker: *const ()) {
        Self::wake_by_ref(waker);
        Self::drop(waker);
    }

    unsafe fn wake_by_ref(waker: *const ()) {
        let waker = waker as *const FutureResponseWaker;
        (*waker).executor.wake((*waker).future_idx);
    }

    unsafe fn drop(waker: *const ()) {
        drop(Box::from_raw(waker as *mut () as *mut FutureResponseWaker));
    }
}

pub struct Executor {
    // TODO: Can we use a lockless data structure for the list without requiring a fixed size?
    futures:       Mutex<Vec<Option<Pin<Box<dyn Future<Output = ()>+Send>>>>>,
    // TODO: Can we use a lockless data structure for the queue without requiring a fixed size?
    awake_futures: Mutex<Vec<usize>>,   // A queue of indices into `futures`
    futures_left:  AtomicUsize          // The number of `Some` entries left in `futures`
}

#[derive(Debug)]
pub enum ExecutorState {
    Finished,   // All futures have returned Poll::Ready
    Pending     // At least one future has not returned Poll::Ready
}

impl Executor {
    const fn new() -> Executor {
        Executor {
            futures: Mutex::new(Vec::new()),
            awake_futures: Mutex::new(Vec::new()),
            futures_left: AtomicUsize::new(0)
        }
    }

    pub fn spawn<F: 'static+Future<Output = ()>+Send>(&self, future: F) -> &Executor {
        // FIXME: Spawning a thread from asynchronous code (e.g. when an `Image` is dropped)
        // causes a deadlock because of this mutex.
        let mut futures = self.futures.lock();
        self.awake_futures.lock().push(futures.len());
        futures.push(Some(Box::pin(future)));
        self.futures_left.fetch_add(1, Ordering::SeqCst);

        self
    }

    /// Polls each future that is currently awake, then reports on whether there are any left pending.
    /// Because this only ever runs any given future once, futures can freely keep themselves
    /// awake (by calling `cx.waker().wake_by_ref()`) without risking deadlock.
    pub fn execute(&self) -> ExecutorState {
        let orig_len = self.awake_futures.lock().len();

        // Poll each future once that is currently awake.
        for _ in 0 .. orig_len {
            let idx = self.awake_futures.lock().remove(0);

            let done = match self.futures.lock()[idx] {
                Some(ref mut future) => {
                    let waker = unsafe {
                        Waker::from_raw(
                            RawWaker::new(
                                Box::into_raw(Box::new(FutureResponseWaker::new(self, idx))) as *const (),
                                &FUTURE_RESPONSE_RAW_WAKER_VTABLE
                            )
                        )
                    };
                    match Future::poll(future.as_mut(), &mut Context::from_waker(&waker)) {
                        Poll::Ready(()) => true,
                        Poll::Pending   => false
                    }
                },
                None => false   // `true` would decrement `self.futures_left`.
            };
            if done {
                self.futures.lock()[idx] = None;
                self.futures_left.fetch_sub(1, Ordering::Release);
            }
        }

        if self.futures_left.load(Ordering::Acquire) == 0 {
            ExecutorState::Finished
        } else {
            ExecutorState::Pending
        }
    }

    /// Repeatedly executes all futures until they all finish. Needless to say, this can deadlock.
    pub fn execute_blocking(&self) {
        loop {
            if let ExecutorState::Finished = self.execute() {
                break;
            }
        }
    }

    fn wake(&self, future_idx: usize) {
        self.awake_futures.lock().push(future_idx);
    }
}

// ************************************
// GPU-Specific VirtIO Device Operation
// ************************************

static RESOURCES_2D: Vec<Option<Image>> = Vec::new();

const MAX_SCANOUTS: usize = 16; // This is given in the specification.

ffi_enum! {
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum GPUCommType {
        // 2D commands
        CmdGetDisplayInfo           = u32::to_le(0x0100), // Get information about all the scanouts
        CmdResourceCreate2D         = u32::to_le(0x0101), // Make a new 2D resource
        CmdResourceUnref            = u32::to_le(0x0102), // Delete a resource
        CmdSetScanout               = u32::to_le(0x0103),
        CmdResourceFlush            = u32::to_le(0x0104),
        CmdTransferToHost2D         = u32::to_le(0x0105),
        CmdResourceAttachBacking    = u32::to_le(0x0106), // Attach a resource to some backing memory
        CmdResourceDetachBacking    = u32::to_le(0x0107), // Detach a resource from its backing memory
        CmdGetCapsetInfo            = u32::to_le(0x0108),
        CmdGetCapset                = u32::to_le(0x0109),
        CmdGetEDID                  = u32::to_le(0x010a), // Get a scanout's VESA EDID blob (if feature flag set)

        // Cursor commands (best to use the cursor queue for these)
        CmdUpdateCursor             = u32::to_le(0x0300), // Set cursor image and position
        CmdMoveCursor               = u32::to_le(0x0301), // Set cursor position

        // Success responses
        RespOKNoData                = u32::to_le(0x1100), // No data, just success
        RespOKDisplayInfo           = u32::to_le(0x1101), // Information on scanouts
        RespOKCapsetInfo            = u32::to_le(0x1102),
        RespOKCapset                = u32::to_le(0x1103),
        RespOKEDID                  = u32::to_le(0x1104), // A scanout's VESA EDID blob

        // Error responses
        RespErrUnspec               = u32::to_le(0x1200), // Miscellaneous error
        RespErrOutOfMemory          = u32::to_le(0x1201),
        RespErrInvalidScanoutID     = u32::to_le(0x1202),
        RespErrInvalidResourceID    = u32::to_le(0x1203),
        RespErrInvalidContextID     = u32::to_le(0x1204),
        RespErrInvalidParameter     = u32::to_le(0x1205)
    }
}

bitflags! {
    struct GPUCommFlags: u32 {
        const FENCE = u32::to_le(1); // Forces the device to finish the operation before responding
    }
}

#[derive(Debug)]
#[repr(C)]
struct GPUControlQHeader {
    comm_type: GPUCommType,
    flags:     GPUCommFlags,
    fence_id:  Le<u64>, // Shouldn't matter what this is since we use a promise-based interface
    ctx_id:    Le<u32>, // Unused in 2D mode
    padding:   Le<u32>
}

impl GPUControlQHeader {
    pub fn new(comm_type: GPUCommType, flags: GPUCommFlags) -> GPUControlQHeader {
        GPUControlQHeader {
            comm_type,
            flags,
            fence_id: Le::<u64>::from_native(0),
            ctx_id:   Le::<u32>::from_native(0),
            padding:  Le::<u32>::from_native(0)
        }
    }
}

// Asynchronously sends a command to the GPU and waits for the response.
fn send_recv_gpu<T: GPUCmd+?Sized+Debug>(
        msg: Box<T>,
        virtq: &'static VirtQueue,
        response_type: GPUCommType,
        legacy_response_len: usize
) -> impl Future<Output = Result<Response<Box<T>>, ()>> {
    struct SendRecvFuture<T: GPUCmd+?Sized+Debug> {
        msg:                 Option<Box<T>>,
        response_offset:     usize,
        response_type:       GPUCommType,
        virtq:               &'static VirtQueue,
        inner_future:        Option<FutureResponse<'static, Box<T>>>,
        legacy_response_len: usize
    }
    impl<T: GPUCmd+?Sized+Debug> Future for SendRecvFuture<T> {
        type Output = Result<Response<Box<T>>, ()>;
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<Response<Box<T>>, ()>> {
            let self_response_offset = self.response_offset;
            let self_response_type = self.response_type;
            let self_legacy_response_len = self.legacy_response_len;

            if self.inner_future.is_none() {
                // Send the command.
                let msg = mem::replace(&mut self.msg, None);
                match self.virtq.send_recv(msg.unwrap(), self_response_offset, self_legacy_response_len) {
                    Ok(Ok(response_future)) => self.inner_future = Some(response_future),
                    Ok(Err(msg)) => {
                        // The virtqueue is probably full. Try again the next time the executor runs.
                        self.msg = Some(msg);
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    },
                    Err(x) => panic!("failed to send a virtio GPU command: {}", x)
                }
            }

            // Wait for the response.
            match unsafe { self.map_unchecked_mut(|s| s.inner_future.as_mut().unwrap()) }.poll(cx) {
                Poll::Ready(msg) => {
                    assert!(
                        msg.valid_bytes >= self_response_offset,
                        "fewer valid bytes in command+response than sent in the command"
                    );

                    let response_type = unsafe {
                        *((&*msg.value_ptr as *const _ as *const u8).add(self_response_offset) as *const u32)
                    };
                    match GPUCommType::try_from(response_type) {
                        Ok(response_type) if response_type == self_response_type => Poll::Ready(Ok(msg)),
                        _                                                        => {
                            // The GPU returned an error, or at least an unexpected type of response.
                            // TODO: Return some error information.
                            Poll::Ready(Err(()))
                        }
                    }
                },
                Poll::Pending => Poll::Pending
            }
        }
    }
    let response_offset = msg.response_offset();
    SendRecvFuture::<T> {
        msg:          Some(msg),
        response_offset,
        response_type,
        virtq,
        inner_future: None,
        legacy_response_len
    }
}

trait GPUCmd {
    fn response_offset(&self) -> usize;
    fn response_type(&self) -> GPUCommType;
}

#[derive(Debug)]
#[repr(C)]
struct CmdGetDisplayInfo {
    header:   GPUControlQHeader,
    response: RespOKDisplayInfo
}

impl CmdGetDisplayInfo {
    pub fn new() -> Box<CmdGetDisplayInfo> {
        let flags = GPUCommFlags::empty();
        Box::new(CmdGetDisplayInfo {
            header:   GPUControlQHeader::new(GPUCommType::CmdGetDisplayInfo, flags),
            response: RespOKDisplayInfo::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdGetDisplayInfo {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdResourceCreate2D {
    header:      GPUControlQHeader,
    resource_id: Le<u32>,
    format:      Resource2DFormat,
    width:       Le<u32>,
    height:      Le<u32>,
    response:    RespOKNoData
}

#[derive(Debug)]
#[repr(u32)]
enum Resource2DFormat {
    BytesBGRA = u32::to_le(0x01),
    BytesBGRX = u32::to_le(0x02),
    BytesARGB = u32::to_le(0x03),
    BytesXRGB = u32::to_le(0x04),

    BytesRGBA = u32::to_le(0x43),
    BytesXBGR = u32::to_le(0x44),

    BytesABGR = u32::to_le(0x79),
    BytesRGBX = u32::to_le(0x86)
}

impl CmdResourceCreate2D {
    // `resource_id` must be non-zero
    pub fn new(resource_id: u32, format: Resource2DFormat, width: u32, height: u32, flags: GPUCommFlags) -> Box<CmdResourceCreate2D> {
        Box::new(CmdResourceCreate2D {
            header:      GPUControlQHeader::new(GPUCommType::CmdResourceCreate2D, flags),
            resource_id: Le::<u32>::from_native(resource_id),
            format,
            width:       Le::<u32>::from_native(width),
            height:      Le::<u32>::from_native(height),
            response:    RespOKNoData::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdResourceCreate2D {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdResourceUnref {
    header:      GPUControlQHeader,
    resource_id: Le<u32>,
    padding:     Le<u32>,
    response:    RespOKNoData
}

impl CmdResourceUnref {
    // `resource_id` must be non-zero
    pub fn new(resource_id: u32) -> Box<CmdResourceUnref> {
        Box::new(CmdResourceUnref {
            header:      GPUControlQHeader::new(GPUCommType::CmdResourceUnref, GPUCommFlags::empty()),
            resource_id: Le::<u32>::from_native(resource_id),
            padding:     Le::<u32>::from_native(0),
            response:    RespOKNoData::new(GPUCommFlags::FENCE)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdResourceUnref {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdSetScanout {
    header:      GPUControlQHeader,
    rect:        LeRectangle,
    scanout_id:  Le<u32>,
    resource_id: Le<u32>,
    response:    RespOKNoData
}

impl CmdSetScanout {
    // Set `resource_id` to zero to disable the scanout
    pub fn new(scanout_id: u32, resource_id: u32, rect: Rectangle, flags: GPUCommFlags) -> Box<CmdSetScanout> {
        Box::new(CmdSetScanout {
            header:      GPUControlQHeader::new(GPUCommType::CmdSetScanout, flags),
            rect:        rect.into(),
            scanout_id:  Le::<u32>::from_native(scanout_id),
            resource_id: Le::<u32>::from_native(resource_id),
            response:    RespOKNoData::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdSetScanout {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdResourceFlush {
    header:      GPUControlQHeader,
    rect:        LeRectangle,
    resource_id: Le<u32>,
    padding:     Le<u32>,
    response:    RespOKNoData
}

impl CmdResourceFlush {
    // `resource_id` must be non-zero
    pub fn new(resource_id: u32, rect: Rectangle, flags: GPUCommFlags) -> Box<CmdResourceFlush> {
        Box::new(CmdResourceFlush {
            header:      GPUControlQHeader::new(GPUCommType::CmdResourceFlush, flags),
            rect:        rect.into(),
            resource_id: Le::<u32>::from_native(resource_id),
            padding:     Le::<u32>::from_native(0),
            response:    RespOKNoData::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdResourceFlush {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdTransferToHost2D {
    header:      GPUControlQHeader,
    rect:        LeRectangle,
    dest_offset: Le<u64>,
    resource_id: Le<u32>,
    padding:     Le<u32>,
    response:    RespOKNoData
}

impl CmdTransferToHost2D {
    // `resource_id` must be non-zero
    pub fn new(resource_id: u32, rect: Rectangle, dest_offset: u64, flags: GPUCommFlags) -> Box<CmdTransferToHost2D> {
        Box::new(CmdTransferToHost2D {
            header:      GPUControlQHeader::new(GPUCommType::CmdTransferToHost2D, flags),
            rect:        rect.into(),
            dest_offset: Le::<u64>::from_native(dest_offset),
            resource_id: Le::<u32>::from_native(resource_id),
            padding:     Le::<u32>::from_native(0),
            response:    RespOKNoData::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdTransferToHost2D {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdResourceAttachBacking {
    header:      GPUControlQHeader,
    resource_id: Le<u32>,
    entries_len: Le<u32>,
    entries:     [u8] // Contains the array of `MemEntries` and the response
}

#[derive(Debug)]
#[repr(C)]
struct MemEntry {
    base:    Le<u64>,
    size:    Le<u32>,
    padding: Le<u32>
}

impl CmdResourceAttachBacking {
    // `resource_id` must be non-zero.
    pub fn new(resource_id: u32, entries: &[&[u8]], flags: GPUCommFlags) -> Box<CmdResourceAttachBacking> {
        // Allocate space for the command on the heap.
        let size =
            mem::size_of::<GPUControlQHeader>() +
            2 * mem::size_of::<u32>() +
            entries.len() * mem::size_of::<MemEntry>() +
            mem::size_of::<RespOKNoData>();
        let layout = Layout::from_size_align(size, mem::align_of::<u64>()).unwrap();
        let mut boxed = unsafe {
            let thin_ptr: *mut u8 = alloc(layout);
            let fat_ptr = ptr::slice_from_raw_parts_mut(thin_ptr, size) as *mut CmdResourceAttachBacking;
            Box::from_raw(fat_ptr)
        };

        // Initialize the command.
        mem::forget(mem::replace(&mut boxed.header, GPUControlQHeader::new(GPUCommType::CmdResourceAttachBacking, flags)));
        mem::forget(mem::replace(&mut boxed.resource_id, Le::<u32>::from_native(resource_id)));
        // TODO: The hardware expects physical addresses. Not important for now since we're
        // identity-mapping, but it will be important when we move this to userspace.
        mem::forget(mem::replace(&mut boxed.entries_len, Le::<u32>::from_native(entries.len() as u32)));
        for i in 0 .. boxed.entries_len.into_native() as usize {
            let entry_dest = unsafe { &mut *(boxed.entries.as_mut_ptr() as *mut MemEntry).add(i) };
            mem::forget(mem::replace(entry_dest, MemEntry {
                base:    Le::<u64>::from_native(entries[i] as *const [u8] as *const u8 as u64),
                size:    Le::<u32>::from_native(mem::size_of_val(entries[i]) as u32),
                padding: Le::<u32>::from_native(0)
            }));
            assert_eq!(entries[i] as *const [u8] as *const u8, entry_dest.base.into_native() as *const u8);
            assert_eq!(mem::size_of_val(entries[i]), entry_dest.size.into_native() as usize);
        }
        let response_dest = unsafe {
            &mut *((boxed.entries.as_mut_ptr() as *mut MemEntry).add(boxed.entries_len.into_native() as usize) as *mut RespOKNoData)
        };
        mem::forget(mem::replace(response_dest, RespOKNoData::new(flags)));

        boxed
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        let self_size = mem::size_of_val(&*self);
        send_recv_gpu(
            self,
            control_q,
            response_type,
            self_size
        ).await
    }

    fn response(&self) -> &RespOKNoData {
        unsafe { &*((self as *const _ as *const u8).add(self.response_offset()) as *const RespOKNoData) }
    }
}

impl GPUCmd for CmdResourceAttachBacking {
    fn response_offset(&self) -> usize {
        let entries_offset = unsafe { (&self.entries as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize };
        entries_offset + self.entries_len.into_native() as usize * mem::size_of::<MemEntry>()
    }

    fn response_type(&self) -> GPUCommType {
        self.response().header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdResourceDetachBacking {
    header:      GPUControlQHeader,
    resource_id: Le<u32>,
    padding:     Le<u32>,
    response:    RespOKNoData
}

impl CmdResourceDetachBacking {
    // `resource_id` must be non-zero
    pub fn new(resource_id: u32, flags: GPUCommFlags) -> Box<CmdResourceDetachBacking> {
        Box::new(CmdResourceDetachBacking {
            header:      GPUControlQHeader::new(GPUCommType::CmdResourceDetachBacking, flags),
            resource_id: Le::<u32>::from_native(resource_id),
            padding:     Le::<u32>::from_native(0),
            response:    RespOKNoData::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let control_q = CONTROL_Q.try_get().expect("virtio GPU: no control queue");
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            control_q,
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CmdResourceDetachBacking {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

// TODO: Implement this command if needed.
// #[derive(Debug)]
// #[repr(C)]
// struct CmdGetEDID { ... }

#[derive(Debug)]
#[repr(C)]
struct CursorPosition {
    scanout_id: Le<u32>,
    x:          Le<u32>,
    y:          Le<u32>,
    padding:    Le<u32>
}

impl CursorPosition {
    pub fn new(scanout_id: u32, x: u32, y: u32) -> CursorPosition {
        CursorPosition {
            scanout_id: Le::<u32>::from_native(scanout_id),
            x:          Le::<u32>::from_native(x),
            y:          Le::<u32>::from_native(y),
            padding:    Le::<u32>::from_native(0)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct CursorCommand {
    header:      GPUControlQHeader,
    position:    CursorPosition,
    resource_id: Le<u32>,
    hot_x:       Le<u32>,
    hot_y:       Le<u32>,
    padding:     Le<u32>,
    response:    RespOKNoData
}

impl CursorCommand {
    pub fn new_update(position: CursorPosition, resource_id: u32, hot_x: u32, hot_y: u32, flags: GPUCommFlags) -> Box<CursorCommand> {
        Box::new(CursorCommand {
            header:      GPUControlQHeader::new(GPUCommType::CmdUpdateCursor, flags),
            position,
            resource_id: Le::<u32>::from_native(resource_id),
            hot_x:       Le::<u32>::from_native(hot_x),
            hot_y:       Le::<u32>::from_native(hot_y),
            padding:     Le::<u32>::from_native(0),
            response:    RespOKNoData::new(flags)
        })
    }

    pub fn new_move(position: CursorPosition, flags: GPUCommFlags) -> Box<CursorCommand> {
        Box::new(CursorCommand {
            header:      GPUControlQHeader::new(GPUCommType::CmdMoveCursor, flags),
            position,
            resource_id: Le::<u32>::from_native(0),
            hot_x:       Le::<u32>::from_native(0),
            hot_y:       Le::<u32>::from_native(0),
            padding:     Le::<u32>::from_native(0),
            response:    RespOKNoData::new(flags)
        })
    }

    async fn send_recv(self: Box<Self>) -> Result<Response<Box<Self>>, ()> {
        let response_type = self.response_type();
        send_recv_gpu(
            self,
            CURSOR_Q.try_get().expect("virtio GPU: no cursor queue"),
            response_type,
            mem::size_of::<Self>()
        ).await
    }
}

impl GPUCmd for CursorCommand {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> GPUCommType {
        self.response.header.comm_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct RespOKNoData {
    header: GPUControlQHeader
}

impl RespOKNoData {
    /// Returns an unspecified error. If the device succeeds, it will overwrite this with the
    /// correct response.
    fn new(flags: GPUCommFlags) -> RespOKNoData {
        RespOKNoData {
            header: GPUControlQHeader::new(GPUCommType::RespOKNoData, flags)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct RespOKDisplayInfo {
    header:   GPUControlQHeader,
    displays: [RawDisplayInfo; MAX_SCANOUTS]
}

impl RespOKDisplayInfo {
    /// Returns an unspecified error. If the device succeeds, it will overwrite this with the
    /// correct response.
    fn new(flags: GPUCommFlags) -> RespOKDisplayInfo {
        RespOKDisplayInfo {
            header: GPUControlQHeader::new(GPUCommType::RespOKDisplayInfo, flags),
            displays: array![RawDisplayInfo::default(); 16]
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct RawDisplayInfo {
    rect:    LeRectangle, // The display's physical position and size
    enabled: Le<u32>,     // Should be interpreted as a boolean like it would be in C
    flags:   SingleDisplayInfoFlags
}

impl Default for RawDisplayInfo {
    fn default() -> Self {
        Self {
            rect: LeRectangle {
                x: Le::<u32>::from_native(0),
                y: Le::<u32>::from_native(0),
                width: Le::<u32>::from_native(0),
                height: Le::<u32>::from_native(0)
            },
            enabled: Le::<u32>::from_native(0),
            flags: SingleDisplayInfoFlags::empty()
        }
    }
}

bitflags! {
    struct SingleDisplayInfoFlags: u32 {
        // The specification doesn't actually define any flags here.
        const UNDEFINED = 0;
    }
}

#[derive(Debug)]
#[repr(C)]
struct RespErr {
    header: GPUControlQHeader
}

#[derive(Debug)]
pub struct Rectangle {
    x:      u32,
    y:      u32,
    width:  u32,
    height: u32
}

impl Rectangle {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Rectangle {
        Rectangle { x, y, width, height }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct LeRectangle {
    x:      Le<u32>,
    y:      Le<u32>,
    width:  Le<u32>,
    height: Le<u32>
}

impl From<Rectangle> for LeRectangle {
    fn from(rect: Rectangle) -> LeRectangle {
        LeRectangle {
            x:      Le::<u32>::from_native(rect.x),
            y:      Le::<u32>::from_native(rect.y),
            width:  Le::<u32>::from_native(rect.width),
            height: Le::<u32>::from_native(rect.height)
        }
    }
}

impl From<LeRectangle> for Rectangle {
    fn from(rect: LeRectangle) -> Rectangle {
        Rectangle {
            x:      rect.x.into_native(),
            y:      rect.y.into_native(),
            width:  rect.width.into_native(),
            height: rect.height.into_native()
        }
    }
}

// **********************
// API and Implementation
// **********************

static ACTIVE_DISPLAY_RES_IDX: AtomicUsize = AtomicUsize::new(0);

/// This type represents a tile in an image. It's really just a number, and the driver has no idea
/// what it means. In a text mode, for instance, this would be the numeric representation of a
/// character, most likely in ASCII. For any image understood by the VirtIO GPU there is only one
/// possible tile (a solid-colored pixel), so every bit is ignored when writing, and zero is
/// returned when reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Tile(u32);

/// This type represents a color in an image. This is either paletted or truecolor, and in the
/// latter case it can be stored in any format (as long as it's not bigger than this type). The
/// driver doesn't really care what the format is; it just copies the color bitwise into an image.
/// The graphics library is responsible for producing colors in the correct format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Color(u32);

#[derive(Debug)]
pub struct DisplayInfo {
    rect:  Rectangle,
    flags: DisplayInfoFlags
}

impl DisplayInfo {
    pub async fn all() -> Result<Vec<DisplayInfo>, ()> {
        let response = &mut CmdGetDisplayInfo::new().send_recv().await?;
        let valid_data_bytes = response.valid_bytes - unsafe {
            (&response.value_ptr.response.displays as *const _ as *const u8)
                .offset_from(&*response.value_ptr as *const _ as *const u8) as usize
        };
        let raw = &mut response.value_ptr.response.displays[0 .. valid_data_bytes / mem::size_of::<RawDisplayInfo>()];
        let mut displays = Vec::with_capacity(raw.len());
        for raw_display in raw {
            let mut flags = DisplayInfoFlags::empty();
            if raw_display.enabled.into_native() != 0 { flags |= DisplayInfoFlags::ENABLED; }

            displays.push(DisplayInfo {
                rect: Rectangle::from(mem::replace(&mut raw_display.rect, Default::default())),
                flags
            });
        }
        Ok(displays)
    }

    pub async fn one(idx: usize) -> Result<DisplayInfo, ()> {
        match Self::all().await {
            Ok(mut all) if all.len() > idx => {
                Ok(all.swap_remove(idx))
            },
            _ => {
                // TODO: Return some error information on failure.
                Err(())
            }
        }
    }
}

bitflags! {
    pub struct DisplayInfoFlags: u32 {
        const ENABLED = 0x1;
    }
}

/// Returns `Ok` if the image is accepted as the display's framebuffer and `Err` if it's
/// incompatible.
pub async fn set_display_framebuffer(display_idx: u32, fb: &Image) -> Result<(), ()> {
    if let Some(resource_id) = fb.resource_id {
        CmdSetScanout::new(display_idx, resource_id.get(), fb.rect(), GPUCommFlags::FENCE)
            .send_recv().await?;
        Ok(()) // TODO: Return some error information on failure.
    } else {
        // The image couldn't be given to the GPU when it was created, probably because of its
        // format.
        Err(())
    }
}

#[derive(Debug)]
pub struct Image {
    tile_length:     u8, // Bytes per tile (for specifying which tile, not the colors)
    colors_per_tile: u8,
    bits_per_color:  u8,
    bytes_per_color: u8, // Equal to bits_per_color / 8, rounded up

    width:           u32, // Measured in tiles
    //height:          u32, // Measured in tiles (can be calculated as needed)

    resource_id:     Option<NonZeroU32>, // `Some` if the GPU has a copy of this image, else `None`
    backing:         Vec<u8>,

    drawing_colors:  Vec<Color> // The colors that will be used to draw new tiles
}

impl Image {
    /// Returns `Ok` with a image containing the given settings if they work with the hardware.
    /// Otherwise returns `Err`. Also provides the image to the GPU as a 2D resource if
    /// `as_gpu_resource` is `true`, allowing it to, e.g., be used as a framebuffer.
    pub async fn new(as_gpu_resource: bool, tile_length: u8, colors_per_tile: u8, bits_per_color: u8, width: u32, height: u32)
            -> Result<Image, ()> {
        // TODO: Instead of doing this (insufficient) check at the beginning, catch any unwraps
        // that happen and return `Err(())` in that case. Otherwise, we can't
        // catch a failure in `Vec::with_capacity` or `slice::repeat`. Unfortunately, we can't do
        // this when panics are set to "abort", so we'll need to get unwrapping working.
        if width > 4096 || height > 4096 // Larger screens can exist, but they should have dedicated VRAM.
                || tile_length as usize > mem::size_of::<Tile>()
                || bits_per_color as usize > mem::size_of::<Color>() * 8 {
            return Err(());
        }

        let bytes_per_color = (bits_per_color + 7) / 8;
        let backing_len = ((tile_length as u32 + colors_per_tile as u32 * bytes_per_color as u32) * width * height) as usize;

        let mut img = Image {
            tile_length,
            colors_per_tile,
            bits_per_color,
            bytes_per_color,
            width,
            resource_id: None,
            backing: [0].repeat(backing_len),
            drawing_colors: Vec::with_capacity(colors_per_tile as usize)
        };
        img.drawing_colors.resize_with(colors_per_tile as usize, Default::default);

        if as_gpu_resource && img.virtio_compatible() { img.resource_id = alloc_resource(); }

        if let Some(resource_id) = img.resource_id {
            CmdResourceCreate2D::new(resource_id.get(), Resource2DFormat::BytesRGBA, width, height, GPUCommFlags::FENCE)
                .send_recv().await?;
            CmdResourceAttachBacking::new(resource_id.get(), &[&*img.backing], GPUCommFlags::FENCE)
                .send_recv().await?;
            Ok(img) // TODO: Send back some error information on failure.
        } else {
            Ok(img)
        }
    }

    /// Determines whether the VirtIO GPU can store this image as a Resource2D.
    fn virtio_compatible(&self) -> bool {
        self.tile_length() == 0
            && self.colors_per_tile() == 1
            && self.bits_per_color() == 32
    }

    /// Sends the given part of the image to the host.
    pub async fn send_rect(&self, rect: Rectangle) -> Result<(), ()> {
        if let Some(resource_id) = self.resource_id {
            let bytes_per_tile = self.tile_length as u64 + self.colors_per_tile as u64 * self.bytes_per_color as u64;
            let dest = (rect.x + rect.y * self.width) as u64 * bytes_per_tile;
            CmdTransferToHost2D::new(resource_id.get(), rect, dest, GPUCommFlags::FENCE).send_recv().await?;
            // TODO: Send back some error information on failure.
        }
        Ok(())
    }

    /// Equivalent to `self.send_rect(self.rect())`.
    pub async fn send_all(&self) -> Result<(), ()> {
        self.send_rect(self.rect()).await
    }

    /// Flushes the given part of the image to the screen. Note that this has an effect only if the
    /// image is the backing for a scanout.
    pub async fn flush_rect(&self, rect: Rectangle) -> Result<(), ()> {
        if let Some(resource_id) = self.resource_id {
            CmdResourceFlush::new(resource_id.get(), rect, GPUCommFlags::FENCE).send_recv().await?;
            // TODO: Send back some error information on failure.
        }
        Ok(())
    }

    /// Equivalent to `self.flush_rect(self.rect())`.
    pub async fn flush_all(&self) -> Result<(), ()> {
        self.flush_rect(self.rect()).await
    }

    fn rect(&self) -> Rectangle {
        Rectangle::new(0, 0, self.width(), self.height())
    }

    fn width(&self) -> u32 { self.width }

    fn height(&self) -> u32 {
        let bytes_per_tile = self.tile_length() as u32 + self.colors_per_tile() as u32 * self.bytes_per_color as u32;
        let bytes_per_row = bytes_per_tile * self.width();
        self.backing.len() as u32 / bytes_per_row
    }

    fn tile_length(&self) -> u8 { self.tile_length }

    fn colors_per_tile(&self) -> u8 { self.colors_per_tile }

    fn bits_per_color(&self) -> u8 { self.bits_per_color }

    /// Sets a range of colors to be used when drawing new tiles to this image (intended to be used
    /// when setting all colors at the same time).
    fn set_colors(&mut self, first_index: usize, colors: &[Color]) {
        if first_index >= self.drawing_colors.len() {
            return;
        }
        let upper_bound = if first_index + colors.len() <= self.drawing_colors.len() {
            colors.len()
        } else {
            self.drawing_colors.len() - first_index
        };
        for i in 0 .. upper_bound {
            self.drawing_colors[first_index + i] = colors[i];
        }
    }

    /// Returns the array of colors currently being used for drawing new tiles.
    fn colors(&self) -> &[Color] { &*self.drawing_colors }

    /// Returns the tile found at the given coordinates.
    fn tile(&self, x: u32, y: u32) -> Option<(Tile, Vec<Color>)> {
        if x < self.width() && y < self.height() {
            let bytes_per_tile = self.tile_length() as u32 + self.colors_per_tile() as u32 * self.bytes_per_color as u32;
            let idx = ((x + y * self.width()) * bytes_per_tile) as usize;
            let mut tile_bytes = [0u8; 4];
            for i in 0 .. self.tile_length() as usize {
                tile_bytes[i] = self.backing[idx + i];
            }
            let tile = Tile(u32::from_ne_bytes(tile_bytes));

            let mut colors = [Color(0)].repeat(self.colors_per_tile as usize);
            for i in 0 .. self.colors_per_tile as usize {
                let mut color_bytes = [0u8; 4];
                for j in 0 .. self.bytes_per_color as usize {
                    color_bytes[j] = self.backing[idx + self.tile_length() as usize + self.bytes_per_color as usize * i + j];
                }
                colors[i] = Color(u32::from_ne_bytes(color_bytes)
                    & ((1 << self.bits_per_color()) - 1));   // Ignore any unused bits in the last byte.
            }

            Some((tile, colors))
        } else {
            None
        }
    }

    /// Draws a rectangle filled with the given tile.
    fn draw_rect_region(&mut self, mut rect: Rectangle, tile: Tile) {
        // Stay within the image's bounds.
        let self_width = self.width();
        let self_height = self.height();
        let tile_length = self.tile_length();
        let colors_per_tile = self.colors_per_tile();
        let bytes_per_color = self.bytes_per_color;
        let drawing_colors = &self.drawing_colors;

        if rect.x + rect.width > self_width {
            rect.width = self_width - rect.x;
        }
        if rect.y + rect.height > self_height {
            rect.height = self_height - rect.y;
        }
        let bytes_per_tile = tile_length as usize + self.colors_per_tile as usize * self.bytes_per_color as usize;

        for y in rect.y .. rect.y + rect.height {
            for x in rect.x .. rect.x + rect.width {
                let start = (x + y * self_width) as usize * bytes_per_tile;
                let tile_backing = &mut self.backing[start .. start + bytes_per_tile as usize];
                let mut tile = tile.0;
                for i in 0 .. tile_length as usize {
                    tile_backing[i] = (tile & 0xff) as u8;
                    tile >>= 8;
                }
                for i in 0 .. colors_per_tile as usize {
                    let mut color = drawing_colors[i].0;
                    for j in 0 .. bytes_per_color as usize {
                        tile_backing[tile_length as usize + i * bytes_per_color as usize + j] = (color & 0xff) as u8;
                        color >>= 8;
                    }
                }
            }
        }
    }

    // TODO: Add functions for drawing other primitives, like lines, circles, and ellipses.
    // TODO: The real driver will probably also want a batch drawing function to reduce the
    // overhead of repeated system calls and IPC.

    /// Blits the given tiles to the given rectangle. Like the C standard library's `memcpy`,
    /// this function isn't safe for copying between overlapping parts of the same image. Use
    /// `move_rect` for that.
    ///
    /// # Returns
    /// `Ok` if the blit succeeded. `Err` if not, probably because the images use different formats
    /// (tileset size, color depth, and number of colors per tile).
    fn blit(&mut self, src: &Image, src_rect: Rectangle, dest_x: u32, dest_y: u32) -> Result<(), ()> {
        if src.tile_length() != self.tile_length() || src.colors_per_tile() != self.colors_per_tile()
                || src.bits_per_color() != self.bits_per_color() {
            return Err(());
        }

        // Stay within the source image's bounds.
        let src_x = src_rect.x;
        let src_y = src_rect.y;
        let src_width = src.width();
        let src_height = src.height();
        let mut width = src_width;
        let mut height = src_height;
        if src_x + width > src_width {
            width = src_width - src_x;
        }
        if src_y + height > src_height {
            height = src_height - src_y;
        }
        // Stay within the destination image's bounds.
        let dest_width = self.width();
        let dest_height = self.height();
        if dest_x + width > dest_width {
            width = dest_width - dest_x;
        }
        if dest_y + height > dest_height {
            height = dest_height - dest_y;
        }

        let bytes_per_tile = self.tile_length as usize + self.colors_per_tile as usize * self.bytes_per_color as usize;
        let bytes_per_row = width as usize * bytes_per_tile;

        let src_idx = (src_x + src_y * src_width) as usize * bytes_per_tile;
        let dest_idx = (dest_x + dest_y * dest_width) as usize * bytes_per_tile;
        for dy in 0 .. height as usize {
            for i in 0 .. bytes_per_row {
                self.backing[dest_idx + dest_width as usize * bytes_per_tile * dy + i]
                    = src.backing[src_idx + src_width as usize * bytes_per_tile * dy + i];
            }
        }

        Ok(())
    }

    /// Blits the given tiles from one rectangle to another rectangle of the same size. Like
    /// the C standard library's `memmove`, this function is somewhat slower than `blit` (the
    /// equivalent of `memcpy`) but guarantees a correct copy even if the rectangles overlap.
    fn move_rect(&mut self, src_rect: Rectangle, dest_x: u32, dest_y: u32) {
        // If the destination is either above the source or directly to the left, a normal blit
        // will work. Otherwise, a reversed blit (starting at the bottom-right corner) will work.

        let self_width = self.width();
        let self_height = self.height();

        // Stay within the image's bounds.
        let src_x = src_rect.x;
        let src_y = src_rect.y;
        let mut width = self_width;
        let mut height = self_height;
        if src_x > self_width || src_y > self_height || dest_x > self_width || dest_y > self_height {
            return;
        }
        if src_x + width > self_width {
            width = self_width - src_x;
        }
        if src_y + height > self_height {
            height = self_height - src_y;
        }
        if dest_x + width > self_width {
            width = self_width - dest_x;
        }
        if dest_y + height > self_height {
            height = self_height - src_y;
        }

        #[derive(Clone)]
        enum DirectionalRange {
            Up(Range<u32>),
            Down(Rev<Range<u32>>)
        }
        impl Iterator for DirectionalRange {
            type Item = u32;
            fn next(&mut self) -> Option<Self::Item> {
                match *self {
                    Self::Up(ref mut range) => range.next(),
                    Self::Down(ref mut rev) => rev.next()
                }
            }
        }

        let (dx_range, dy_range);

        if dest_x + dest_y * self_width <= src_x + src_y * self_width {
            dx_range = DirectionalRange::Up(0 .. width);
            dy_range = DirectionalRange::Up(0 .. height);
        } else {
            dx_range = DirectionalRange::Down((0 .. width).rev());
            dy_range = DirectionalRange::Down((0 .. height).rev());
        }

        let bytes_per_tile = self.tile_length as usize + self.colors_per_tile as usize * self.bytes_per_color as usize;

        for dy in dy_range {
            for dx in dx_range.clone() {
                let src_idx = ((src_x + dx) + (src_y + dy) * self_width) as usize * bytes_per_tile;
                let dest_idx = ((dest_x + dx) + (dest_y + dy) * self_width) as usize * bytes_per_tile;
                for i in 0 .. bytes_per_tile {
                    self.backing[dest_idx + i] = self.backing[src_idx + i];
                }
            }
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if let Some(resource_id) = self.resource_id {
            EXECUTOR.spawn(async move {
                if let Err(()) = CmdResourceUnref::new(resource_id.get()).send_recv().await {
                    // TODO: Log an error, but don't panic. This is just a memory leak.
                }
                free_resource(resource_id);
            });
        }
    }
}

const MAX_RESOURCES: usize = 4096; // Must be a power of 2 and fit in a u32
static RESOURCE_IDS_USED: [AtomicBool; MAX_RESOURCES] = array![AtomicBool::new(false); 4096];

/// Allocates an ID for a new Resource2D to be given to the GPU.
fn alloc_resource() -> Option<NonZeroU32> {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0); // Zero-based even though the returned ID will be one-based

    // Search for and claim the first slot, starting at NEXT_ID, that is empty. If there is none, return None.
    // Also update NEXT_ID to make the next run more efficient.
    RESOURCE_IDS_USED.iter()
        .enumerate()
        .cycle() // We treat the array as circular.
        .skip(NEXT_ID.fetch_add(1, Ordering::AcqRel)) // Start at NEXT_ID. `cycle` takes care of the modular arithmetic.
        .take(MAX_RESOURCES) // We need to stop sometime.
        .find(|(_, x)| !x.swap(true, Ordering::AcqRel))
        .map(|(i, _)| unsafe { NonZeroU32::new_unchecked(i as u32 + 1) }) // Zero-based index to one-based ID
}

/// Frees a Resource2D ID so it can be reused. (This doesn't communicate with the GPU.)
/// Panics if that ID wasn't already allocated.
fn free_resource(id: NonZeroU32) {
    let i = id.get() - 1;
    assert!(RESOURCE_IDS_USED[i as usize].swap(false, Ordering::AcqRel));
}
