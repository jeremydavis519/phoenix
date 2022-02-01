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

//! This module defines a generic virtqueue data structure, as described by the VirtIO specification.
//! It is the core data structure underlying the entire VirtIO communication protocol.

pub mod future;

use {
    alloc::{
        boxed::Box,
        vec::Vec
    },
    core::{
        cell::RefCell,
        convert::TryInto,
        iter,
        mem,
        ops::{Index, IndexMut},
        ptr,
        slice,
        sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering},
        task::Waker
    },
    bitflags::bitflags,
    libdriver::Resource,
    libphoenix::{
        allocator::{Allocator, PhysBox},
        syscall
    },
    crate::{DeviceEndian, VirtIoError},
    self::future::ResponseFuture
};

/// A virtqueue, as defined in the VirtIO specification. This queue is the primary means of
/// communication between the device and its driver.
#[derive(Debug)]
pub struct VirtQueue<'a> {
    resource: &'a Resource,
    device_features: u64,
    id: u32,
    descriptors: DescriptorTable,
    driver_ring: DriverRing,
    device_ring: DeviceRing,
    last_dev_ring_idx: AtomicU16,
    accumulated_batch_size: AtomicU16, // Used for handling the `IN_ORDER` feature
    wakers: Box<[RefCell<Option<Waker>>]>,
    legacy: bool
}

impl<'a> VirtQueue<'a> {
    // FIXME: This depends on the transport, so it may not always be 0x1000.
    pub(crate) const LEGACY_DEVICE_RING_ALIGN: usize = 0x1000;

    pub(crate) fn new(
            resource: &'a Resource,
            device_features: u64,
            legacy: bool,
            id: u32,
            len: u16,
            driver_flags: DriverFlags
    ) -> Self {
        let len = len as usize;
        let page_size = syscall::memory_page_size();

        // Base-2 logarithm, rounded down
        let log_2 = |x: usize| mem::size_of_val(&x) * 8 - x.leading_zeros() as usize + 1;

        let descriptors;
        let driver_ring;
        let device_ring;
        if legacy {
            // In "legacy" devices, everything needs to be roughly contiguous, so we allocate it
            // all in one chunk.
            let size_of_descriptors = mem::size_of::<BufferDescriptor>() * len;
            let size_of_driver_ring = mem::size_of::<u16>() * (3 + len);
            let size_of_device_ring = mem::size_of::<u16>() * 3 + mem::size_of::<UsedElem>() * len;
            let align = |x| (x + Self::LEGACY_DEVICE_RING_ALIGN - 1) & !(Self::LEGACY_DEVICE_RING_ALIGN - 1);
            let block = Allocator.malloc_phys_bytes(
                align(size_of_descriptors + size_of_driver_ring) + align(size_of_device_ring),
                usize::max(Self::LEGACY_DEVICE_RING_ALIGN, page_size),
                32 + log_2(page_size)
            )
                .expect("failed to allocate a virtqueue");
            unsafe {
                driver_ring = DriverRing::new_legacy(&block, size_of_descriptors, len, driver_flags);
                device_ring = DeviceRing::new_legacy(&block, align(size_of_descriptors + size_of_driver_ring), len);
            }
            descriptors = DescriptorTable::new_legacy(block, len);
        } else {
            // TODO
            unimplemented!();
        }

        let mut wakers = Vec::with_capacity(len);
        wakers.resize_with(len, || RefCell::new(None));
        let wakers = wakers.into_boxed_slice();

        VirtQueue {
            resource,
            device_features,
            id,
            descriptors,
            driver_ring,
            device_ring,
            last_dev_ring_idx: AtomicU16::new(0),
            accumulated_batch_size: AtomicU16::new(0),
            wakers,
            legacy
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.descriptors.len
    }

    pub(crate) fn descriptors_addr_phys(&self) -> usize {
        self.descriptors.base_addr_phys()
    }

    pub(crate) fn driver_ring_addr_phys(&self) -> usize {
        self.driver_ring.base_addr_phys()
    }

    pub(crate) fn device_ring_addr_phys(&self) -> usize {
        self.device_ring.base_addr_phys()
    }

    /// Asynchronously sends a message to the device and returns its response.
    ///
    /// # Parameters
    /// * `buf`: A buffer that contains the message and has room to append a response.
    /// * `first_recv_idx`: The byte offset from the beginning of `buf` where the response will begin.
    /// * `legacy_response_len`: The length to assume if the device doesn't report a length with its
    ///     response (needed for some legacy devices). Leave this as `None` if no suitable assumption
    ///     can be made.
    ///
    /// # Returns
    /// A [`SendRecvResult`]. See its definition for a description of each variant. The payload on
    /// success is a future that will evaluate to `buf` after the response is placed in it. If the
    /// queue is full, `buf` is returned immediately with no changes. If an error occurs, only an
    /// error object is returned.
    pub fn send_recv<T: ?Sized>(&self, buf: PhysBox<T>, first_recv_idx: usize, legacy_response_len: Option<usize>)
            -> SendRecvResult<ResponseFuture<T>, PhysBox<T>, VirtIoError> {
        let buf_size = mem::size_of_val(&*buf);

        if buf_size > u32::MAX as usize {
            return SendRecvResult::Err(
                VirtIoError::new("attempted to write a buffer of at least 4 GiB to a virtqueue")
            );
        }

        // If we're supposed to send and receive zero bytes, return immediately.
        if buf_size == 0 {
            return SendRecvResult::Ok(ResponseFuture::new_immediate(buf));
        }

        // We need one descriptor for output and one for input.
        // If `first_recv_idx` is past the end of `buf`, we're only outputting.
        // If it's `0`, we're only inputting.
        let mut descriptor_indices = [0u16; 2];
        let descriptor_indices = &mut descriptor_indices[
            if first_recv_idx >= buf_size || first_recv_idx == 0 { 0 .. 1 } else { 0 .. 2 }
        ];

        match self.descriptors.make_chain(descriptor_indices, self.legacy) {
            SendRecvResult::Ok(()) => {},
            SendRecvResult::Retry(()) => return SendRecvResult::Retry(buf),
            SendRecvResult::Err(e) => return SendRecvResult::Err(e)
        };

        // Attach the descriptors to the appropriate parts of the buffer.
        if first_recv_idx > 0 {
            let first_desc = &self.descriptors[0];
            first_desc.set_addr(buf.addr_phys() as u64, self.legacy);
            first_desc.set_len(usize::min(buf_size, first_recv_idx) as u32, self.legacy);
        }
        if first_recv_idx < buf_size {
            let last_desc = &self.descriptors[descriptor_indices.len() - 1];
            last_desc.set_addr((buf.addr_phys() + first_recv_idx) as u64, self.legacy);
            last_desc.set_len((buf_size - first_recv_idx) as u32, self.legacy);

            // Mark this as an input buffer (i.e. writable from the device's perspective).
            last_desc.set_flags(last_desc.flags(self.legacy) | BufferFlags::WRITE, self.legacy);
        }

        // The device only needs the index of the first descriptor in the chain.
        self.driver_ring.set_next_entry(descriptor_indices[0]);

        // Notify the device of the new buffers, but only if it expects notifications.
        if !self.device_ring.flags().contains(DeviceFlags::NO_INTERRUPT) {
            super::notify_device(self.resource, self.id);
        }

        // Wait for the device to respond.
        SendRecvResult::Ok(ResponseFuture::new(
            self,
            descriptor_indices[0],
            descriptor_indices[descriptor_indices.len() - 1],
            descriptor_indices.len().try_into().unwrap(),
            buf,
            legacy_response_len
        ))
    }
}

/// The type returned by [`VirtQueue::send_recv`]. This is distinct from the usual `Result` type in
/// that it allows for a middle ground between success and failure, where the caller should just
/// retry the operation at a later time.
#[derive(Debug)]
pub enum SendRecvResult<O, R, E> {
    /// Indicates success.
    Ok(O),
    /// Indicates a recoverable failure.
    Retry(R),
    /// Indicates an unrecoverable failure.
    Err(E)
}

bitflags! {
    /// The flags that can be stored in the driver ring.
    pub struct DriverFlags: u16 {
        /// Indicates that the driver does not require an interrupt from the device to notify it
        /// when buffers are consumed.
        const NO_INTERRUPT = 0x0001;
    }
}

bitflags! {
    struct DeviceFlags: u16 {
        const NO_INTERRUPT = 0x0001;
    }
}

#[derive(Debug)]
struct DescriptorTable {
    descriptors: DescriptorTableInternal,
    len: usize,
    free_descs: AtomicU16,
    first_free_idx: AtomicU16 // Stored in device-endian order
}

#[derive(Debug)]
enum DescriptorTableInternal {
    Legacy(PhysBox<[u8]>),
    Modern(PhysBox<[BufferDescriptor]>)
}

impl DescriptorTable {
    fn new_legacy(block: PhysBox<[u8]>, len: usize) -> Self {
        DescriptorTable {
            descriptors: DescriptorTableInternal::Legacy(block),
            len,
            free_descs: AtomicU16::new(len as u16),
            first_free_idx: AtomicU16::new(0)
        }.clear(true)
    }

    fn new_modern(len: usize) -> Self {
        let block: PhysBox<[BufferDescriptor]> = Allocator.malloc_phys_array(len, 64)
            .expect("failed to allocate a virtqueue");
        DescriptorTable {
            descriptors: DescriptorTableInternal::Modern(block),
            len,
            free_descs: AtomicU16::new(len as u16),
            first_free_idx: AtomicU16::new(0)
        }.clear(false)
    }

    fn clear(mut self, legacy: bool) -> Self {
        let len = self.len;
        assert!(len < u16::max_value() as usize);
        for i in 0 .. len {
            mem::forget(mem::replace(
                &mut self[i],
                BufferDescriptor::new(0, 0, BufferFlags::empty(), (i + 1) as u16, legacy)
            ));
        }
        self
    }

    fn base_addr_phys(&self) -> usize {
        match self.descriptors {
            DescriptorTableInternal::Legacy(ref phys_box) => phys_box.addr_phys(),
            DescriptorTableInternal::Modern(ref phys_box) => phys_box.addr_phys()
        }
    }

    // Disconnects and returns a chain of descriptors from the list of free descriptors. These
    // descriptors will have their flags set to `BufferFlags::NEXT` if needed (and will thus
    // describe output buffers) and their `next` pointers set appropriately, but no buffers will
    // actually be attached. Be sure to attach the buffers and modify the flags as needed before
    // sending the chain to the device.
    //
    // # Arguments
    // `descriptor_indices`: A slice containing the indices of all the descriptors in the chain.
    //   This will be populated by the function; only the length of the slice functions as input.
    //
    // # Returns
    // A `SendRecvResult` indicating whether we can continue the send-receive operation, may be able
    // to do it in the future, or will never be able to do it.
    fn make_chain(&self, descriptor_indices: &mut [u16], legacy: bool) -> SendRecvResult<(), (), VirtIoError> {
        if descriptor_indices.len() == 0 {
            return SendRecvResult::Ok(());
        }

        if descriptor_indices.len() > self.len {
            return SendRecvResult::Err(
                VirtIoError::new("attempted to make a chain with more descriptors than the queue has")
            );
        }

        // Decrease the number of free descriptors by the number requested. If we can do that
        // without underflowing, we're guaranteed to find enough that are available.
        let mut free_descs = self.free_descs.load(Ordering::Acquire);
        let requested_descs = descriptor_indices.len() as u16;
        loop {
            if free_descs < requested_descs {
                return SendRecvResult::Retry(());
            }
            match self.free_descs.compare_exchange(
                    free_descs,
                    free_descs - requested_descs,
                    Ordering::AcqRel,
                    Ordering::Acquire
            ) {
                Ok(_) => break,
                Err(x) => free_descs = x
            };
        }

        // Claim the descriptors.
        for i in 0 .. descriptor_indices.len() {
            let next = &self.first_free_idx;
            let mut idx = next.load(Ordering::Acquire);

            loop {
                let idx_next = self[u16::from_device_endian(idx, legacy) as usize].next(Ordering::Acquire, legacy);
                assert_ne!(idx_next as usize, self.len);
                match next.compare_exchange(
                    idx,
                    idx_next,
                    Ordering::AcqRel,
                    Ordering::Acquire
                ) {
                    Ok(_) => break,
                    Err(x) => idx = x
                };
            }

            descriptor_indices[i] = u16::from_device_endian(idx, legacy);
        }

        // Set up the `next` pointers.
        for i in 0 .. descriptor_indices.len() - 1 {
            self[i].set_flags(BufferFlags::NEXT, legacy);
            self[i].next.store(descriptor_indices[i + 1].to_device_endian(legacy), Ordering::Release);
        }
        self[descriptor_indices.len() - 1].set_flags(BufferFlags::empty(), legacy);

        SendRecvResult::Ok(())
    }

    // TODO: A function for returning a descriptor chain to the free list.
}

impl Index<usize> for DescriptorTable {
    type Output = BufferDescriptor;

    fn index(&self, idx: usize) -> &Self::Output {
        match self.descriptors {
            DescriptorTableInternal::Legacy(ref bytes) => {
                unsafe {
                    &*(&bytes[idx * mem::size_of::<BufferDescriptor>()] as *const u8 as *const BufferDescriptor)
                }
            },
            DescriptorTableInternal::Modern(ref descriptors) => {
                &descriptors[idx]
            }
        }
    }
}

impl IndexMut<usize> for DescriptorTable {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match self.descriptors {
            DescriptorTableInternal::Legacy(ref mut bytes) => {
                unsafe {
                    &mut *(&mut bytes[idx * mem::size_of::<BufferDescriptor>()] as *mut u8 as *mut BufferDescriptor)
                }
            },
            DescriptorTableInternal::Modern(ref mut descriptors) => {
                &mut descriptors[idx]
            }
        }
    }
}

#[derive(Debug)]
#[repr(C, align(16))]
struct BufferDescriptor {
    // Each of these is stored in device-endian order. Use the accessor methods instead.
    addr:  AtomicU64,
    len:   AtomicU32,
    flags: AtomicU16,
    next:  AtomicU16
}

bitflags! {
    struct BufferFlags: u16 {
        const NEXT     = 0x1;
        const WRITE    = 0x2;
        const INDIRECT = 0x4;
    }
}

impl BufferDescriptor {
    fn new(addr: u64, len: u32, flags: BufferFlags, next: u16, legacy: bool) -> Self {
        Self {
            addr:  AtomicU64::new(addr.to_device_endian(legacy)),
            len:   AtomicU32::new(len.to_device_endian(legacy)),
            flags: AtomicU16::new(flags.bits().to_device_endian(legacy)),
            next:  AtomicU16::new(next.to_device_endian(legacy))
        }
    }

    fn addr(&self, legacy: bool) -> u64 {
        u64::from_device_endian(self.addr.load(Ordering::Acquire), legacy)
    }

    fn set_addr(&self, addr: u64, legacy: bool) {
        self.addr.store(addr.to_device_endian(legacy), Ordering::Release);
    }

    fn len(&self, legacy: bool) -> u32 {
        u32::from_device_endian(self.len.load(Ordering::Acquire), legacy)
    }

    fn set_len(&self, len: u32, legacy: bool) {
        self.len.store(len.to_device_endian(legacy), Ordering::Release);
    }

    fn flags(&self, legacy: bool) -> BufferFlags {
        BufferFlags::from_bits(u16::from_device_endian(
            self.flags.load(Ordering::Acquire),
            legacy
        )).unwrap()
    }

    fn set_flags(&self, flags: BufferFlags, legacy: bool) {
        self.flags.store(flags.bits().to_device_endian(legacy), Ordering::Release);
    }

    fn next(&self, ordering: Ordering, legacy: bool) -> u16 {
        u16::from_device_endian(self.next.load(ordering), legacy)
    }
}

#[derive(Debug)]
enum DriverRing {
    Legacy {
        internal: *const DriverRingInternal,
        state:    DriverRingState
    },
    Modern {
        internal: PhysBox<DriverRingInternal>,
        state:    DriverRingState
    }
}

type DriverRingInternal = [AtomicU16];

#[derive(Debug)]
struct DriverRingState {
    next_idx: AtomicU16,
    entries_updated: Vec<AtomicBool>
}

impl DriverRing {
    const FLAGS_OFFSET: usize = 0;
    const IDX_OFFSET:   usize = 1;
    const RING_OFFSET:  usize = 2;

    unsafe fn new_legacy(block: &PhysBox<[u8]>, offset: usize, len: usize, driver_flags: DriverFlags) -> Self {
        let internal = ptr::slice_from_raw_parts(
            &block[offset] as *const u8 as *const AtomicU16,
            len + 3
        );
        (*internal)[Self::FLAGS_OFFSET].store(driver_flags.bits().to_device_endian(true), Ordering::Release);
        (*internal)[Self::IDX_OFFSET].store(0.to_device_endian(true), Ordering::Release);
        (*internal)[Self::RING_OFFSET + len].store(0.to_device_endian(true), Ordering::Release);
        Self::Legacy { internal, state: DriverRingState::new(len) }
    }

    fn legacy(&self) -> bool {
        match *self {
            Self::Legacy { .. } => true,
            Self::Modern { .. } => false
        }
    }

    fn internal(&self) -> &DriverRingInternal {
        match *self {
            Self::Legacy { ref internal, .. } => unsafe { &**internal },
            Self::Modern { ref internal, .. } => &**internal
        }
    }

    fn state(&self) -> &DriverRingState {
        match *self {
            Self::Legacy { ref state, .. } => state,
            Self::Modern { ref state, .. } => state
        }
    }

    fn internal_and_state(&self) -> (&DriverRingInternal, &DriverRingState) {
        match *self {
            Self::Legacy { ref internal, ref state } => (unsafe { &**internal }, state),
            Self::Modern { ref internal, ref state } => (&**internal, state)
        }
    }

    fn base_addr_phys(&self) -> usize {
        match *self {
            Self::Legacy { .. } => panic!("tried to get the base address of a legacy driver ring"),
            Self::Modern { ref internal, .. } => internal.addr_phys()
        }
    }

    fn flags(&self) -> u16 {
        u16::from_device_endian(self.internal()[Self::FLAGS_OFFSET].load(Ordering::Acquire), self.legacy())
    }

    fn idx(&self) -> u16 {
        u16::from_device_endian(self.internal()[Self::IDX_OFFSET].load(Ordering::Acquire), self.legacy())
    }

    fn add_idx(&self, steps: u16) {
        let legacy = self.legacy();
        let idx_de = &self.internal()[Self::IDX_OFFSET];

        if cfg!(not(target_endian = "little")) && !legacy {
            // This device uses little-endian, but the CPU uses big-endian.
            let mut old_idx_le = idx_de.load(Ordering::Acquire);
            loop {
                let old_idx_be = u16::from_le(old_idx_le);
                match idx_de.compare_exchange_weak(
                        old_idx_le,
                        u16::to_le(old_idx_be.wrapping_add(steps)),
                        Ordering::AcqRel,
                        Ordering::Relaxed
                ) {
                    Ok(_) => break,
                    Err(x) => old_idx_le = x
                };
            }
        } else {
            // The device and the CPU share the same endianness.
            idx_de.fetch_add(steps, Ordering::AcqRel);
        }
    }

    fn set_used_event(&self, flags: DriverFlags) {
        self.internal()[self.used_event_offset()].store(flags.bits(), Ordering::Release);
    }

    fn set_next_entry(&self, val: u16) {
        let (internal, state) = self.internal_and_state();

        let mut this_idx = state.next_idx.fetch_add(1, Ordering::AcqRel);
        let entries_updated = &state.entries_updated;
        internal[Self::RING_OFFSET + this_idx as usize % self.len()].store(val, Ordering::Release);

        // `self.idx()` must never skip over an entry that hasn't actually been updated yet. If we
        // are ahead of that device-visible index, just leave a note for the task that's not ahead
        // of it to handle our update for us. If we're not, then handle all those updates, cleaning
        // up the notes as we go.
        assert!(!entries_updated[this_idx as usize % self.len()].swap(true, Ordering::AcqRel));
        if this_idx == self.idx() {
            loop {
                let steps = entries_updated.iter()
                    .cycle() // This is a circular array. No need for `take` because we'll find a `false` within one revolution.
                    .skip(this_idx as usize % self.len())
                    .take_while(|x| x.swap(false, Ordering::AcqRel))
                    .count() as u16;
                self.add_idx(steps);

                // If, between the last `swap` and `add_idx`, a new note was left at the next
                // index, we have to keep going.
                this_idx = this_idx.wrapping_add(steps);
                if !entries_updated[this_idx as usize % self.len()].load(Ordering::Acquire) {
                    break;
                }
            }
        }
    }

    fn used_event_offset(&self) -> usize {
        Self::RING_OFFSET + self.len()
    }

    fn len(&self) -> usize {
        self.internal().len() - 3
    }
}

impl Index<usize> for DriverRing {
    type Output = AtomicU16;

    fn index(&self, i: usize) -> &Self::Output {
        &self.internal()[Self::RING_OFFSET + i]
    }
}

impl DriverRingState {
    fn new(len: usize) -> Self {
        Self {
            next_idx: AtomicU16::new(0),
            entries_updated: iter::repeat(())
                .take(len)
                .map(|()| AtomicBool::new(false))
                .collect()
        }
    }
}

#[derive(Debug)]
enum DeviceRing {
    Legacy(*const DeviceRingInternal),
    Modern(PhysBox<DeviceRingInternal>)
}

type DeviceRingInternal = [u16];

impl DeviceRing {
    const FLAGS_OFFSET: usize = 0;
    const IDX_OFFSET:   usize = 1;
    const RING_OFFSET:  usize = 2;

    unsafe fn new_legacy(block: &PhysBox<[u8]>, offset: usize, len: usize) -> Self {
        let internal = ptr::slice_from_raw_parts_mut(
            &block[offset] as *const u8 as *mut u8 as *mut u16,
            len + 3
        );
        (&mut (*internal)[Self::FLAGS_OFFSET] as *mut u16).write_volatile(0.to_device_endian(true));
        (&mut (*internal)[Self::IDX_OFFSET] as *mut u16).write_volatile(0.to_device_endian(true));
        Self::Legacy(internal)
    }

    fn legacy(&self) -> bool {
        match *self {
            Self::Legacy(_) => true,
            Self::Modern(_) => false
        }
    }

    fn internal(&self) -> &DeviceRingInternal {
        match *self {
            Self::Legacy(ref internal) => unsafe { &**internal },
            Self::Modern(ref internal) => &**internal
        }
    }

    fn base_addr_phys(&self) -> usize {
        match *self {
            Self::Legacy(_) => panic!("tried to get the base address of a legacy device ring"),
            Self::Modern(ref internal) => internal.addr_phys()
        }
    }

    fn flags(&self) -> DeviceFlags {
        DeviceFlags::from_bits_truncate(u16::from_device_endian(
            unsafe { (&self.internal()[Self::FLAGS_OFFSET] as *const u16).read_volatile() },
            self.legacy()
        ))
    }

    fn idx(&self) -> u16 {
        u16::from_device_endian(
            unsafe { (&self.internal()[Self::IDX_OFFSET] as *const u16).read_volatile() },
            self.legacy()
        )
    }

    fn ring(&self) -> &[UsedElem] {
        unsafe {
            slice::from_raw_parts(
                &self.internal()[Self::RING_OFFSET] as *const u16 as *const UsedElem,
                self.len()
            )
        }
    }

    fn avail_event_offset(&self) -> usize {
        Self::RING_OFFSET + self.len()
    }

    fn len(&self) -> usize {
        (self.internal().len() - 3) * mem::size_of::<u16>() / mem::size_of::<UsedElem>()
    }
}

#[derive(Debug)]
#[repr(C)]
struct UsedElem {
    // Each of these is stored in device-endian order. Use the accessor methods instead.
    id:  u32,
    len: u32
}

impl UsedElem {
    fn id(&self, legacy: bool) -> u32 {
        u32::from_device_endian(unsafe { (&self.id as *const u32).read_volatile() }, legacy)
    }

    fn len(&self, legacy: bool) -> u32 {
        u32::from_device_endian(unsafe { (&self.len as *const u32).read_volatile() }, legacy)
    }
}

/// A response from the device.
#[derive(Debug)]
pub struct Response<T: ?Sized> {
    buffer: PhysBox<T>,
    valid_bytes: usize // The number of bytes from the beginning of `*buffer` that are defined
}

impl<T: ?Sized> Response<T> {
    /// Returns the contents of the response. Note that some bytes at the end may be undefined.
    pub fn buffer(&self) -> &PhysBox<T> {
        &self.buffer
    }

    /// Returns the contents of the response. Note that some bytes at the end may be undefined.
    pub fn buffer_mut(&mut self) -> &mut PhysBox<T> {
        &mut self.buffer
    }

    /// Returns the number of bytes that were actually written by the device. Any bytes after these
    /// are undefined.
    pub const fn valid_bytes(&self) -> usize {
        self.valid_bytes
    }
}
