/* Copyright (c) 2021-2022 Jeremy Davis (jeremydavis519@gmail.com)
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
        mem::{self, MaybeUninit},
        ops::{Index, IndexMut},
        ptr,
        slice,
        sync::atomic::{AtomicU8, AtomicU16, AtomicU32, AtomicU64, Ordering},
        task::Waker
    },
    bitflags::bitflags,
    libdriver::Resource,
    libphoenix::{
        allocator::{Allocator, PhysBox},
        syscall
    },
    crate::{DeviceEndian, GenericFeatures, VirtIoError},
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
        let page_size = syscall::memory_page_size();

        // Base-2 logarithm, rounded down
        let log_2 = |x: usize| mem::size_of_val(&x) * 8 - x.leading_zeros() as usize + 1;

        let in_order = device_features & GenericFeatures::IN_ORDER.bits() != 0;

        let descriptors;
        let driver_ring;
        let device_ring;
        if legacy {
            // In "legacy" devices, everything needs to be roughly contiguous, so we allocate it
            // all in one chunk.
            let size_of_descriptors = mem::size_of::<BufferDescriptor>() * usize::from(len);
            let size_of_driver_ring = mem::size_of::<u16>() * (3 + usize::from(len));
            let size_of_device_ring = mem::size_of::<u16>() * 3 + mem::size_of::<UsedElem>() * usize::from(len);
            let align = |x| (x + Self::LEGACY_DEVICE_RING_ALIGN - 1) & !(Self::LEGACY_DEVICE_RING_ALIGN - 1);
            let mut block = Allocator.malloc_phys_bytes(
                align(size_of_descriptors + size_of_driver_ring) + align(size_of_device_ring),
                usize::max(Self::LEGACY_DEVICE_RING_ALIGN, page_size),
                32 + log_2(page_size)
            )
                .expect("failed to allocate a virtqueue");

            unsafe {
                DescriptorTable::init_legacy(&mut block, len);
                DriverRing::init_legacy(&mut block, size_of_descriptors, len, driver_flags);
                DeviceRing::init_legacy(&mut block, align(size_of_descriptors + size_of_driver_ring), len);
            }

            let block = PhysBox::slice_assume_init(block);
            unsafe {
                driver_ring = DriverRing::new_legacy(&block, size_of_descriptors, len);
                device_ring = DeviceRing::new_legacy(&block, align(size_of_descriptors + size_of_driver_ring), len);
            }
            descriptors = DescriptorTable::new_legacy(block, len, in_order);
        } else {
            // TODO
            unimplemented!();
        }

        let mut wakers = Vec::with_capacity(len.into());
        wakers.resize_with(len.into(), || RefCell::new(None));
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

    /// Returns the maximum number of messages that can be waiting in this queue at the same time.
    pub const fn len(&self) -> u16 {
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
            let first_desc = &self.descriptors[descriptor_indices[0]];
            first_desc.set_addr(buf.addr_phys() as u64, self.legacy);
            first_desc.set_len(usize::min(buf_size, first_recv_idx) as u32, self.legacy);
        }
        if first_recv_idx < buf_size {
            let last_desc = &self.descriptors[descriptor_indices[descriptor_indices.len() - 1]];
            last_desc.set_addr((buf.addr_phys() + first_recv_idx) as u64, self.legacy);
            last_desc.set_len((buf_size - first_recv_idx) as u32, self.legacy);

            // Mark this as an input buffer (i.e. writable from the device's perspective).
            last_desc.set_flags(last_desc.flags(self.legacy) | BufferFlags::WRITE, self.legacy);
        }

        // The device only needs the index of the first descriptor in the chain.
        let Ok((idx, entries_revealed)) = self.driver_ring.set_next_entry(descriptor_indices[0]) else {
            return SendRecvResult::Retry(buf);
        };

        let idx_matches_avail_event = || {
            let mut avail_event = self.device_ring.avail_event();
            if avail_event < idx {
                avail_event += self.len() as u16;
            }
            idx <= avail_event && avail_event < idx + entries_revealed
        };

        // Notify the device of the new buffers, but only if it expects notifications.
        let event_index_feature = self.device_features & GenericFeatures::RING_EVENT_INDEX.bits() != 0;
        if (!event_index_feature && !self.device_ring.flags().contains(DeviceFlags::NO_INTERRUPT)) ||
                (event_index_feature && idx_matches_avail_event()) {
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
    descriptors:    DescriptorTableInternal,
    len:            u16,
    free_descs:     AtomicU16,
    first_free_idx: AtomicU16,  // Stored in device-endian order
    in_order:       bool,       // True if the `IN_ORDER` feature was negotiated
}

#[derive(Debug)]
enum DescriptorTableInternal {
    Legacy(PhysBox<[u8]>),
    Modern(PhysBox<[BufferDescriptor]>),
}

impl DescriptorTable {
    // Must be called before `new_legacy` as part of initializing `block`.
    // Unsafe: `block` must be laid out as if it had type `PhysBox<[BufferDescriptor]>`.
    unsafe fn init_legacy(block: &mut PhysBox<[MaybeUninit<u8>]>, len: u16) {
        for i in 0 .. len {
            let desc = &mut block[usize::from(i) * mem::size_of::<BufferDescriptor>()] as *mut _ as *mut MaybeUninit<BufferDescriptor>;
            (*desc).write(
                BufferDescriptor::new(0, 0, BufferFlags::empty(), (i + 1) % len, true)
            );
        }

    }

    fn new_legacy(block: PhysBox<[u8]>, len: u16, in_order: bool) -> Self {
        DescriptorTable {
            descriptors: DescriptorTableInternal::Legacy(block),
            len,
            free_descs: AtomicU16::new(len),
            first_free_idx: AtomicU16::new(0),
            in_order,
        }
    }

    fn new_modern(len: u16, in_order: bool) -> Self {
        let mut block: PhysBox<[MaybeUninit<BufferDescriptor>]> = Allocator.malloc_phys_array(len.into(), 64)
            .expect("failed to allocate a virtqueue");

        for i in 0 .. len {
            block[usize::from(i)].write(BufferDescriptor::new(0, 0, BufferFlags::empty(), (i + 1) % len, false));
        }

        DescriptorTable {
            descriptors: DescriptorTableInternal::Modern(PhysBox::slice_assume_init(block)),
            len,
            free_descs: AtomicU16::new(len),
            first_free_idx: AtomicU16::new(0),
            in_order,
        }
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
    // - `descriptor_indices`: A slice containing the indices of all the descriptors in the chain.
    //   This will be populated by the function; only the length of the slice functions as input.
    // - `in_order`: True if the driver and device negotiated `GenericFeatures::IN_ORDER`.
    // - `legacy`: True if this is a legacy device.
    //
    // # Returns
    // A `SendRecvResult` indicating whether we can continue the send-receive operation, may be able
    // to do it in the future, or will never be able to do it.
    fn make_chain(
        &self,
        descriptor_indices: &mut [u16],
        legacy: bool,
    ) -> SendRecvResult<(), (), VirtIoError> {
        if descriptor_indices.len() == 0 {
            return SendRecvResult::Ok(());
        }

        if descriptor_indices.len() > self.len.into() {
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
                    Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(x) => free_descs = x,
            };
        }

        // Claim the descriptors.
        if self.in_order {
            // The spec doesn't allow the descriptors to be reordered at all when the `IN_ORDER` feature is used.
            let first_idx = if cfg!(target_endian = "little") || legacy {
                self.first_free_idx.fetch_add(descriptor_indices.len() as u16, Ordering::AcqRel)
            } else {
                // Equivalent to `fetch_add`, but for a little-endian value on a big-endian CPU.
                let mut idx = self.first_free_idx.load(Ordering::Acquire);
                loop {
                    match self.first_free_idx.compare_exchange_weak(
                        idx,
                        idx + descriptor_indices.len() as u16,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break,
                        Err(x) => idx = x,
                    }
                }
                idx
            };
            for i in 0 .. descriptor_indices.len() {
                descriptor_indices[i] = (first_idx.wrapping_add(i as u16)) % self.len;
            }
        } else {
            for i in 0 .. descriptor_indices.len() {
                let mut idx = self.first_free_idx.load(Ordering::Acquire);

                loop {
                    let idx_next = self[u16::from_device_endian(idx, legacy)].next(Ordering::Acquire, legacy);
                    assert_ne!(idx_next, self.len);
                    match self.first_free_idx.compare_exchange(
                        idx,
                        idx_next,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => break,
                        Err(x) => idx = x,
                    };
                }

                descriptor_indices[i] = u16::from_device_endian(idx, legacy);
            }
        }

        // Set up the `next` pointers and flags.
        for i in 0 .. descriptor_indices.len() - 1 {
            let idx = descriptor_indices[i];
            self[idx].set_flags(BufferFlags::NEXT, legacy);
            if !self.in_order { // If in_order, the `next` pointers are always correct.
                self[idx].next.store(descriptor_indices[i + 1].to_device_endian(legacy), Ordering::Release);
            }
        }
        let idx = descriptor_indices[descriptor_indices.len() - 1];
        self[idx].set_flags(BufferFlags::empty(), legacy);

        SendRecvResult::Ok(())
    }

    // Returns a descriptor chain to the list of free descriptors.
    //
    // # Arguments
    // - `head_idx`: The index of the first descriptor in the chain.
    // - `tail_idx`: The index of the last descriptor in the chain.
    // - `count`: The number of descriptors in the chain.
    fn dealloc_chain(&self, head_idx: u16, tail_idx: u16, count: u16) {
        assert!(count > 0);
        if !self.in_order { // Nothing to do if the descriptors are used in order, since they're already connected.
            let mut next = self.first_free_idx.load(Ordering::Acquire); // Device-endian
            loop {
                let tail = &self[tail_idx];
                tail.next.store(next, Ordering::Release);
                match self.first_free_idx.compare_exchange_weak(
                    next,
                    head_idx,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(x) => next = x // The list has a new head. Retry with that one.
                }
            }
        }
        self.free_descs.fetch_add(count, Ordering::AcqRel);
    }
}

impl Index<u16> for DescriptorTable {
    type Output = BufferDescriptor;

    fn index(&self, idx: u16) -> &Self::Output {
        match self.descriptors {
            DescriptorTableInternal::Legacy(ref bytes) => {
                unsafe { &*(
                    &bytes[usize::from(idx) * mem::size_of::<BufferDescriptor>()]
                        as *const u8 as *const BufferDescriptor
                ) }
            },
            DescriptorTableInternal::Modern(ref descriptors) => {
                &descriptors[usize::from(idx)]
            },
        }
    }
}

impl IndexMut<u16> for DescriptorTable {
    fn index_mut(&mut self, idx: u16) -> &mut Self::Output {
        match self.descriptors {
            DescriptorTableInternal::Legacy(ref mut bytes) => {
                unsafe { &mut *(
                    &mut bytes[usize::from(idx) * mem::size_of::<BufferDescriptor>()]
                        as *mut u8 as *mut BufferDescriptor
                ) }
            },
            DescriptorTableInternal::Modern(ref mut descriptors) => {
                &mut descriptors[usize::from(idx)]
            },
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
    next:  AtomicU16,
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
            next:  AtomicU16::new(next.to_device_endian(legacy)),
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
            legacy,
        )).unwrap()
    }

    fn set_flags(&self, flags: BufferFlags, legacy: bool) {
        self.flags.store(flags.bits().to_device_endian(legacy), Ordering::Release);
    }

    fn next(&self, ordering: Ordering, legacy: bool) -> u16 {
        u16::from_device_endian(self.next.load(ordering), legacy)
    }
}

// The spec calls this the "available ring".
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
    entries_flags: Vec<AtomicU8>,
}

bitflags! {
    struct DriverRingEntryFlags: u8 {
        // Allows lock-free insertions
        const UPDATED   = 0x01;
        // An implementation detail to avoid a data race
        const PROTECTED = 0x02;
    }
}

impl DriverRing {
    const FLAGS_OFFSET: usize = 0;
    const IDX_OFFSET:   usize = 1;
    const RING_OFFSET:  usize = 2;

    // Must be called before `new_legacy` as part of initializing `block`.
    // Safety: `block` must be laid out as if it had type `PhysBox<[AtomicU16]>`.
    unsafe fn init_legacy(block: &mut PhysBox<[MaybeUninit<u8>]>, offset: usize, len: u16, driver_flags: DriverFlags) {
        let internal = slice::from_raw_parts_mut(
            &mut block[offset] as *mut _ as *mut MaybeUninit<AtomicU16>,
            usize::from(len) + 3,
        );
        internal[Self::FLAGS_OFFSET].write(AtomicU16::new(driver_flags.bits().to_device_endian(true)));
        internal[Self::IDX_OFFSET].write(AtomicU16::new(0.to_device_endian(true)));
        internal[Self::RING_OFFSET + usize::from(len)].write(AtomicU16::new(0.to_device_endian(true)));
    }

    unsafe fn new_legacy(block: &PhysBox<[u8]>, offset: usize, len: u16) -> Self {
        let internal = ptr::slice_from_raw_parts(
            &block[offset] as *const _ as *const AtomicU16,
            usize::from(len) + 3,
        );
        Self::Legacy { internal, state: DriverRingState::new(len) }
    }

    fn legacy(&self) -> bool {
        match *self {
            Self::Legacy { .. } => true,
            Self::Modern { .. } => false,
        }
    }

    fn internal(&self) -> &DriverRingInternal {
        match *self {
            Self::Legacy { ref internal, .. } => unsafe { &**internal },
            Self::Modern { ref internal, .. } => &**internal,
        }
    }

    fn state(&self) -> &DriverRingState {
        match *self {
            Self::Legacy { ref state, .. } => state,
            Self::Modern { ref state, .. } => state,
        }
    }

    fn internal_and_state(&self) -> (&DriverRingInternal, &DriverRingState) {
        match *self {
            Self::Legacy { ref internal, ref state } => (unsafe { &**internal }, state),
            Self::Modern { ref internal, ref state } => (&**internal, state),
        }
    }

    fn base_addr_phys(&self) -> usize {
        match *self {
            Self::Legacy { .. } => panic!("tried to get the base address of a legacy driver ring"),
            Self::Modern { ref internal, .. } => internal.addr_phys(),
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
                        Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => old_idx_le = x,
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

    // Adds an entry to the ring and returns the index and the number of entries that have been
    // made available to the device by this call.
    fn set_next_entry(&self, val: u16) -> Result<(u16, u16), DriverRingNextEntryError> {
        let (internal, state) = self.internal_and_state();
        let entries_flags = &state.entries_flags;

        let this_idx = match state.next_idx.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |x| {
                let flags = DriverRingEntryFlags::from_bits(
                    entries_flags[x as usize % self.len()].load(Ordering::Acquire),
                ).unwrap();
                if flags.contains(DriverRingEntryFlags::PROTECTED) {
                    // The entry is currently being used.
                    None
                } else {
                    Some(x + 1)
                }
            },
        ) {
            Ok(x) => x,
            Err(_) => return Err(DriverRingNextEntryError),
        };

        internal[Self::RING_OFFSET + this_idx as usize % self.len()].store(val, Ordering::Release);

        // `self.idx()` must never skip over an entry that hasn't actually been updated yet. If we
        // are ahead of that device-visible index, just leave a note for the task that's not ahead
        // of it to handle our update for us. If we're not, then handle all those updates, cleaning
        // up the notes as we go.
        assert_eq!(
            entries_flags[this_idx as usize % self.len()].swap(DriverRingEntryFlags::UPDATED.bits(), Ordering::AcqRel),
            DriverRingEntryFlags::empty().bits(),
        );
        let mut entries_revealed = 0;
        let finalize = this_idx == self.idx();
        if finalize {
            // Note: Only one task can ever do any work in this loop at a time, although that
            // guarantee isn't obvious. If two tasks are ever in the loop at the same time, they
            // will both be at the same index. One will clean up the first note, and the other will
            // immediately exit. This all depends on avoiding the potential race condition noted
            // below.
            let mut next_idx = this_idx;
            loop {
                let steps = entries_flags.iter()
                    .cycle() // This is a circular array. No need for `take` because we'll find a `false` within one revolution.
                    .skip(next_idx as usize % self.len())
                    .take_while(|x| {
                        let bits = x.fetch_and(!DriverRingEntryFlags::UPDATED.bits(), Ordering::AcqRel);
                        DriverRingEntryFlags::from_bits(bits).unwrap().contains(DriverRingEntryFlags::UPDATED)
                    })
                    .count() as u16;
                if steps == 0 { break; }

                // `DriverRingEntryFlags::PROTECTED` prevents a race condition here. It ensures that
                // `state.next_idx` can't wrap around the value this task expects to be in `self.idx()`,
                // which keeps this task at the back of the pack. If a new note is found after
                // `self.add_idx`, the only way it could have gotten there is that another task allocated
                // the very next descriptor and saw that `self.idx()` hadn't been incremented yet.
                next_idx = next_idx.wrapping_add(steps);
                let flags = &entries_flags[next_idx as usize % self.len()];
                assert_eq!(
                    flags.fetch_or(DriverRingEntryFlags::PROTECTED.bits(), Ordering::AcqRel),
                    DriverRingEntryFlags::empty().bits(),
                );
                self.add_idx(steps);
                let new_note = DriverRingEntryFlags::from_bits(flags.load(Ordering::Acquire)).unwrap()
                    .contains(DriverRingEntryFlags::UPDATED);
                assert_eq!(
                    flags.fetch_and(!DriverRingEntryFlags::PROTECTED.bits(), Ordering::AcqRel),
                    DriverRingEntryFlags::PROTECTED.bits(),
                );

                entries_revealed += steps;

                // If, between the last `fetch_and` and `add_idx`, a new note was left at the next
                // index, we have to keep going.
                if !new_note {
                    break;
                }
            }
        }

        Ok((this_idx, entries_revealed))
    }

    fn used_event_offset(&self) -> usize {
        Self::RING_OFFSET + self.len()
    }

    fn len(&self) -> usize {
        self.internal().len() - 3
    }
}

impl Index<u16> for DriverRing {
    type Output = AtomicU16;

    fn index(&self, i: u16) -> &Self::Output {
        &self.internal()[Self::RING_OFFSET + usize::from(i)]
    }
}

impl DriverRingState {
    fn new(len: u16) -> Self {
        Self {
            next_idx: AtomicU16::new(0),
            entries_flags: iter::repeat(())
                .take(len.into())
                .map(|()| AtomicU8::new(DriverRingEntryFlags::empty().bits()))
                .collect(),
        }
    }
}

struct DriverRingNextEntryError;

// The spec calls this the "used ring".
#[derive(Debug)]
enum DeviceRing {
    Legacy(*const DeviceRingInternal),
    Modern(PhysBox<DeviceRingInternal>),
}

type DeviceRingInternal = [u16];

impl DeviceRing {
    const FLAGS_OFFSET: usize = 0;
    const IDX_OFFSET:   usize = 1;
    const RING_OFFSET:  usize = 2;

    // Must be called before `new_legacy` as part of initializing `block`.
    // Safety: `block` must be laid out as if it had type `PhysBox<[u16]>`.
    unsafe fn init_legacy(block: &mut PhysBox<[MaybeUninit<u8>]>, offset: usize, len: u16) {
        let internal = slice::from_raw_parts_mut(
            &mut block[offset] as *mut _ as *mut MaybeUninit<u16>,
            usize::from(len) + 3
        );
        internal[Self::FLAGS_OFFSET].as_mut_ptr().write_volatile(0.to_device_endian(true));
        internal[Self::IDX_OFFSET].as_mut_ptr().write_volatile(0.to_device_endian(true));
    }

    unsafe fn new_legacy(block: &PhysBox<[u8]>, offset: usize, len: u16) -> Self {
        let internal = ptr::slice_from_raw_parts_mut(
            &block[offset] as *const _ as *mut u16,
            usize::from(len) + 3
        );
        Self::Legacy(internal)
    }

    fn legacy(&self) -> bool {
        match *self {
            Self::Legacy(_) => true,
            Self::Modern(_) => false,
        }
    }

    fn internal(&self) -> &DeviceRingInternal {
        match *self {
            Self::Legacy(ref internal) => unsafe { &**internal },
            Self::Modern(ref internal) => &**internal,
        }
    }

    fn base_addr_phys(&self) -> usize {
        match *self {
            Self::Legacy(_) => panic!("tried to get the base address of a legacy device ring"),
            Self::Modern(ref internal) => internal.addr_phys(),
        }
    }

    fn flags(&self) -> DeviceFlags {
        DeviceFlags::from_bits_truncate(u16::from_device_endian(
            unsafe {
                (*(&self.internal()[Self::FLAGS_OFFSET] as *const _ as *const AtomicU16))
                    .load(Ordering::Acquire)
            },
            self.legacy(),
        ))
    }

    fn idx(&self) -> u16 {
        u16::from_device_endian(
            unsafe { (&self.internal()[Self::IDX_OFFSET] as *const u16).read_volatile() },
            self.legacy(),
        )
    }

    fn ring(&self) -> &[UsedElem] {
        unsafe {
            slice::from_raw_parts(
                &self.internal()[Self::RING_OFFSET] as *const _ as *const UsedElem,
                self.len(),
            )
        }
    }

    fn avail_event(&self) -> u16 {
        u16::from_device_endian(
            unsafe {
                (*(&self.internal()[self.avail_event_offset()] as *const _ as *const AtomicU16))
                    .load(Ordering::Acquire)
            },
            self.legacy(),
        )
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

    /// Consumes the response and returns the buffer. Note that some bytes at the end may be undefined.
    pub fn into_buffer(self) -> PhysBox<T> {
        self.buffer
    }

    /// Returns the number of bytes that were actually written by the device. Any bytes after these
    /// are undefined.
    pub const fn valid_bytes(&self) -> usize {
        self.valid_bytes
    }
}
