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

//! This module defines all the messages that can be sent between the driver and the GPU.

use {
    alloc::alloc::AllocError,
    core::{
        convert::TryFrom,
        fmt,
        future::Future,
        mem,
        pin::Pin,
        ptr::{self, addr_of, addr_of_mut},
        slice,
        task::{Context, Poll}
    },
    bitflags::bitflags,
    libphoenix::allocator::{Allocator, PhysBox},
    virtio::{
        virtqueue::{
            future::ResponseFuture,
            Response,
            SendRecvResult,
            VirtQueue
        },
        VirtIoError
    },
    crate::MAX_SCANOUTS
};

// *****
//  API
// *****

/// Asynchronously sends a command to the GPU and waits for the response.
pub async fn send_recv_cmd<T: Command+?Sized>(
        mut cmd: PhysBox<T>,
        virtq: &VirtQueue<'_>
) -> Result<Response<T>, GpuError> {
    loop {
        let response_offset = cmd.response_offset();
        let response_type = cmd.response_type();
        let legacy_response_len = mem::size_of_val(&*cmd);
        match virtq.send_recv(cmd, response_offset, Some(legacy_response_len)) {
            SendRecvResult::Ok(future) => {
                let response = future.await;
                if response.valid_bytes() < response_offset {
                    return Err(GpuError::ResponseTooShort(response_type, response.valid_bytes(), response_offset));
                }
                return Ok(response);
            },
            SendRecvResult::Retry(buf) => {
                RelaxFuture::new().await;
                cmd = buf;
            },
            SendRecvResult::Err(e) => return Err(GpuError::VirtIoError(e))
        };
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CmdGetDisplayInfo {
    header:       ControlQHeader,
    pub response: RespOkDisplayInfo
}

impl CmdGetDisplayInfo {
    pub fn new() -> Result<PhysBox<Self>, AllocError> {
        let flags = MsgFlags::empty();
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:   ControlQHeader::new(MsgType::CmdGetDisplayInfo, flags),
            response: RespOkDisplayInfo::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdGetDisplayInfo {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CmdResourceCreate2D {
    header:       ControlQHeader,
    resource_id:  Le32,
    format:       Resource2DFormat,
    width:        Le32,
    height:       Le32,
    pub response: RespOkNoData
}

#[derive(Debug)]
#[repr(u32)]
pub enum Resource2DFormat {
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
    pub fn new(
            resource_id: u32,
            format: Resource2DFormat,
            width: u32,
            height: u32,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdResourceCreate2D, flags),
            resource_id: u32::to_le(resource_id),
            format,
            width:       u32::to_le(width),
            height:      u32::to_le(height),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdResourceCreate2D {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CmdResourceUnref {
    header:       ControlQHeader,
    resource_id:  Le32,
    padding:      Le32,
    pub response: RespOkNoData
}

impl CmdResourceUnref {
    // `resource_id` must be non-zero
    pub fn new(resource_id: u32) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdResourceUnref, MsgFlags::empty()),
            resource_id: u32::to_le(resource_id),
            padding:     u32::to_le(0),
            response:    RespOkNoData::new(MsgFlags::FENCE)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdResourceUnref {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CmdSetScanout {
    header:       ControlQHeader,
    rect:         LeRectangle,
    scanout_id:   Le32,
    resource_id:  Le32,
    pub response: RespOkNoData
}

impl CmdSetScanout {
    // Set `resource_id` to zero to disable the scanout
    pub fn new(
            scanout_id: u32,
            resource_id: u32,
            rect: Rectangle,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdSetScanout, flags),
            rect:        rect.into(),
            scanout_id:  u32::to_le(scanout_id),
            resource_id: u32::to_le(resource_id),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdSetScanout {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CmdResourceFlush {
    header:       ControlQHeader,
    rect:         LeRectangle,
    resource_id:  Le32,
    padding:      Le32,
    pub response: RespOkNoData
}

impl CmdResourceFlush {
    // `resource_id` must be non-zero
    pub fn new(
            resource_id: u32,
            rect: Rectangle,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdResourceFlush, flags),
            rect:        rect.into(),
            resource_id: u32::to_le(resource_id),
            padding:     u32::to_le(0),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdResourceFlush {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CmdTransferToHost2D {
    header:       ControlQHeader,
    rect:         LeRectangle,
    dest_offset:  Le64,
    resource_id:  Le32,
    padding:      Le32,
    pub response: RespOkNoData
}

impl CmdTransferToHost2D {
    // `resource_id` must be non-zero
    pub fn new(
            resource_id: u32,
            rect: Rectangle,
            dest_offset: u64,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdTransferToHost2D, flags),
            rect:        rect.into(),
            dest_offset: u64::to_le(dest_offset),
            resource_id: u32::to_le(resource_id),
            padding:     u32::to_le(0),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdTransferToHost2D {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

// The easier-to-read version
/*#[derive(Debug)]
#[repr(C)]
struct CmdResourceAttachBacking {
    header:      ControlQHeader,
    resource_id: Le32,
    entries_len: Le32,
    entries:     [MemEntry; entries_len],
    response:    RespOkNoData
}*/
// The version that Rust will actually compile (requires accessors)
#[repr(transparent)]
pub struct CmdResourceAttachBacking([u8]);

#[derive(Debug)]
#[repr(C)]
struct MemEntry {
    base:    Le64,
    size:    Le32,
    padding: Le32
}

impl CmdResourceAttachBacking {
    // `mem::align_of::<Self>()` requires `Self: Sized`.
    const ALIGNMENT: usize = 8;

    // `resource_id` must be non-zero.
    pub fn new(
        resource_id: u32,
        entries: &[&PhysBox<[u8]>],
        flags: MsgFlags,
    ) -> Result<PhysBox<Self>, AllocError> {
        // Allocate space for the command on the heap.
        let size =
            mem::size_of::<ControlQHeader>() +
            2 * mem::size_of::<Le32>() +
            entries.len() * mem::size_of::<MemEntry>() +
            mem::size_of::<RespOkNoData>();
        let mut boxed_bytes = Allocator.malloc_phys_bytes(size, Self::ALIGNMENT, 64)?;
        let cmd = &mut *boxed_bytes as *mut _ as *mut Self;

        // Initialize the command.
        unsafe {
            Self::header_ptr_mut(cmd).write(ControlQHeader::new(MsgType::CmdResourceAttachBacking, flags));
            Self::resource_id_ptr_mut(cmd).write(u32::to_le(resource_id));
            Self::entries_len_ptr_mut(cmd).write(u32::to_le(
                u32::try_from(entries.len()).expect("GPU CmdResourceAttachBacking: too many entries")
            ));
            for i in 0 .. u32::from_le(*Self::entries_len_ptr(cmd)) as usize {
                Self::entries_ptr_mut(cmd).get_unchecked_mut(i).write(MemEntry {
                    base:    u64::to_le(
                        u64::try_from(entries[i].addr_phys())
                            .expect("GPU CmdResourceAttachBacking: physical address doesn't fit in 64 bits")
                    ),
                    size:    u32::to_le(
                        u32::try_from(mem::size_of_val(&**entries[i]))
                            .expect("GPU CmdResourceAttachBacking: more than 4 GiB requested in one entry")
                    ),
                    padding: u32::to_le(0),
                });
            }
            Self::response_ptr_mut(cmd).write(RespOkNoData::new(flags));
        }

        let (bytes_ptr, phys_addr) = PhysBox::into_raw(boxed_bytes);
        Ok(PhysBox::from_raw(cmd, phys_addr))
    }

    fn header(&self) -> &ControlQHeader {
        unsafe { &*Self::header_ptr(self) }
    }

    // Unsafe: `ptr` must not be null.
    unsafe fn header_ptr(ptr: *const Self) -> *const ControlQHeader {
        addr_of!((*ptr).0[0]) as *const ControlQHeader
    }

    // Unsafe: `ptr` must not be null.
    unsafe fn header_ptr_mut(ptr: *mut Self) -> *mut ControlQHeader {
        addr_of_mut!((*ptr).0[0]) as *mut ControlQHeader
    }

    fn resource_id(&self) -> &Le32 {
        unsafe { &*Self::resource_id_ptr(self) }
    }

    // Unsafe: `ptr` must not be null.
    unsafe fn resource_id_ptr(ptr: *const Self) -> *const Le32 {
        addr_of!((*ptr).0[mem::size_of::<ControlQHeader>()]) as *const Le32
    }

    // Unsafe: `ptr` must not be null.
    unsafe fn resource_id_ptr_mut(ptr: *mut Self) -> *mut Le32 {
        addr_of_mut!((*ptr).0[mem::size_of::<ControlQHeader>()]) as *mut Le32
    }

    fn entries_len(&self) -> &Le32 {
        unsafe { &*Self::entries_len_ptr(self) }
    }

    // Unsafe: `ptr` must not be null.
    unsafe fn entries_len_ptr(ptr: *const Self) -> *const Le32 {
        addr_of!((*ptr).0[mem::size_of::<ControlQHeader>() + mem::size_of::<Le32>()]) as *const Le32
    }

    // Unsafe: `ptr` must not be null.
    unsafe fn entries_len_ptr_mut(ptr: *mut Self) -> *mut Le32 {
        addr_of_mut!((*ptr).0[mem::size_of::<ControlQHeader>() + mem::size_of::<Le32>()]) as *mut Le32
    }

    fn entries(&self) -> &[MemEntry] {
        unsafe { &*Self::entries_ptr(self) }
    }

    // Unsafe: `(*ptr).entries_len()` must be defined.
    unsafe fn entries_ptr(ptr: *const Self) -> *const [MemEntry] {
        let base_ptr = addr_of!((*ptr).0[mem::size_of::<ControlQHeader>() + 2 * mem::size_of::<Le32>()]) as *const MemEntry;
        ptr::slice_from_raw_parts(base_ptr, usize::try_from(u32::from_le(*(*ptr).entries_len())).unwrap())
    }

    // Unsafe: `(*ptr).entries_len()` must be defined.
    unsafe fn entries_ptr_mut(ptr: *mut Self) -> *mut [MemEntry] {
        let base_ptr = addr_of_mut!((*ptr).0[mem::size_of::<ControlQHeader>() + 2 * mem::size_of::<Le32>()]) as *mut MemEntry;
        ptr::slice_from_raw_parts_mut(base_ptr, usize::try_from(u32::from_le(*(*ptr).entries_len())).unwrap())
    }

    fn response(&self) -> &RespOkNoData {
        unsafe { &*Self::response_ptr(self) }
    }

    // Unsafe: `(*ptr).entries_len()` must be defined.
    unsafe fn response_ptr(ptr: *const Self) -> *const RespOkNoData {
        addr_of!((*ptr).0[Self::response_offset_unsafe(ptr)]) as *const RespOkNoData
    }

    // Unsafe: `(*ptr).entries_len()` must be defined.
    unsafe fn response_ptr_mut(ptr: *mut Self) -> *mut RespOkNoData {
        addr_of_mut!((*ptr).0[Self::response_offset_unsafe(ptr)]) as *mut RespOkNoData
    }

    // Unsafe: `(*ptr).entries_len()` must be defined.
    unsafe fn response_offset_unsafe(ptr: *const Self) -> usize {
        let entries_offset = mem::size_of::<ControlQHeader>() + mem::size_of::<Le32>() * 2;
        entries_offset + usize::try_from(u32::from_le(*(*ptr).entries_len())).unwrap() * mem::size_of::<MemEntry>()
    }
}

impl fmt::Debug for CmdResourceAttachBacking {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CmdResourceAttachBacking (DST)")
            .field("header", self.header())
            .field("resource_id", self.resource_id())
            .field("entries", &self.entries())
            .field("response", self.response())
            .finish()
    }
}

impl Command for CmdResourceAttachBacking {
    fn response_offset(&self) -> usize {
        unsafe { Self::response_offset_unsafe(self) }
    }

    fn response_type(&self) -> MsgType {
        self.response().header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
struct CmdResourceDetachBacking {
    header:       ControlQHeader,
    resource_id:  Le32,
    padding:      Le32,
    pub response: RespOkNoData
}

impl CmdResourceDetachBacking {
    // `resource_id` must be non-zero
    pub fn new(
            resource_id: u32,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdResourceDetachBacking, flags),
            resource_id: u32::to_le(resource_id),
            padding:     u32::to_le(0),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CmdResourceDetachBacking {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

// TODO: Implement this command if needed.
// #[derive(Debug)]
// #[repr(C)]
// struct CmdGetEDID { ... }

#[derive(Debug)]
#[repr(C)]
struct CursorPosition {
    scanout_id: Le32,
    x:          Le32,
    y:          Le32,
    padding:    Le32
}

impl CursorPosition {
    pub fn new(scanout_id: u32, x: u32, y: u32) -> CursorPosition {
        CursorPosition {
            scanout_id: u32::to_le(scanout_id),
            x:          u32::to_le(x),
            y:          u32::to_le(y),
            padding:    u32::to_le(0)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct CursorCommand {
    header:       ControlQHeader,
    position:     CursorPosition,
    resource_id:  Le32,
    hot_x:        Le32,
    hot_y:        Le32,
    padding:      Le32,
    pub response: RespOkNoData
}

impl CursorCommand {
    pub fn new_update(
            position: CursorPosition,
            resource_id: u32,
            hot_x: u32,
            hot_y: u32,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdUpdateCursor, flags),
            position,
            resource_id: u32::to_le(resource_id),
            hot_x:       u32::to_le(hot_x),
            hot_y:       u32::to_le(hot_y),
            padding:     u32::to_le(0),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }

    pub fn new_move(
            position: CursorPosition,
            flags: MsgFlags
    ) -> Result<PhysBox<Self>, AllocError> {
        let mut boxed = Allocator.malloc_phys::<Self>(64)?;
        boxed.write(Self {
            header:      ControlQHeader::new(MsgType::CmdMoveCursor, flags),
            position,
            resource_id: u32::from_le(0),
            hot_x:       u32::from_le(0),
            hot_y:       u32::from_le(0),
            padding:     u32::from_le(0),
            response:    RespOkNoData::new(flags)
        });
        Ok(PhysBox::assume_init(boxed))
    }
}

impl Command for CursorCommand {
    fn response_offset(&self) -> usize {
        unsafe { (&self.response as *const _ as *const u8).offset_from(self as *const _ as *const u8) as usize }
    }

    fn response_type(&self) -> MsgType {
        self.response.header.msg_type
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RespOkNoData {
    header: ControlQHeader
}

impl RespOkNoData {
    /// Returns an unspecified error. If the device succeeds, it will overwrite this with the
    /// correct response.
    fn new(flags: MsgFlags) -> RespOkNoData {
        RespOkNoData {
            header: ControlQHeader::new(MsgType::RespOkNoData, flags)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RespOkDisplayInfo {
    header:       ControlQHeader,
    pub displays: [RawDisplayInfo; MAX_SCANOUTS]
}

impl RespOkDisplayInfo {
    /// Returns an unspecified error. If the device succeeds, it will overwrite this with the
    /// correct response.
    fn new(flags: MsgFlags) -> RespOkDisplayInfo {
        RespOkDisplayInfo {
            header: ControlQHeader::new(MsgType::RespOkDisplayInfo, flags),
            displays: [const { RawDisplayInfo::new() }; 16]
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RawDisplayInfo {
    pub rect:    LeRectangle, // The display's physical position and size
    pub enabled: Le32,        // Should be interpreted as a boolean like it would be in C
    pub flags:   SingleDisplayInfoFlags
}

impl RawDisplayInfo {
    const fn new() -> Self {
        Self {
            rect: LeRectangle {
                x: u32::to_le(0),
                y: u32::to_le(0),
                width: u32::to_le(0),
                height: u32::to_le(0)
            },
            enabled: u32::to_le(0),
            flags: SingleDisplayInfoFlags::empty()
        }
    }
}

impl Default for RawDisplayInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    pub struct SingleDisplayInfoFlags: u32 {
        // The specification doesn't actually define any flags here.
        const UNDEFINED = 0;
    }
}

#[derive(Debug)]
pub struct Rectangle {
    pub x:      u32,
    pub y:      u32,
    pub width:  u32,
    pub height: u32
}

impl Rectangle {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Rectangle {
        Rectangle { x, y, width, height }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct LeRectangle {
    x:      Le32,
    y:      Le32,
    width:  Le32,
    height: Le32
}

impl From<Rectangle> for LeRectangle {
    fn from(rect: Rectangle) -> LeRectangle {
        LeRectangle {
            x:      u32::to_le(rect.x),
            y:      u32::to_le(rect.y),
            width:  u32::to_le(rect.width),
            height: u32::to_le(rect.height)
        }
    }
}

impl From<LeRectangle> for Rectangle {
    fn from(rect: LeRectangle) -> Rectangle {
        Rectangle {
            x:      u32::from_le(rect.x),
            y:      u32::from_le(rect.y),
            width:  u32::from_le(rect.width),
            height: u32::from_le(rect.height)
        }
    }
}


// ***********
//  Internals
// ***********

// These type aliases show when numbers are expected to be in little-endian order. (Newtypes would
// be safer, but also bulkier.)
type Le32 = u32;
type Le64 = u64;

// A future that returns `Pending` once, then `Ready`. The purpose is to allow other futures to run
// while an `async` block waits for an external event.
struct RelaxFuture {
    finished: bool
}

impl RelaxFuture {
    const fn new() -> Self {
        Self { finished: false }
    }
}

impl Future for RelaxFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
        if self.finished {
            Poll::Ready(())
        } else {
            self.finished = true;
            Poll::Pending
        }
    }
}

/// Any type that represents a command that can be sent to the GPU.
pub trait Command {
    /// The offset in the structure where the device's response begins.
    fn response_offset(&self) -> usize;
    /// The expected (or, if the GPU has already responded, actual) type of the response.
    fn response_type(&self) -> MsgType;
}

#[derive(Debug)]
#[repr(C)]
struct ControlQHeader {
    msg_type: MsgType,
    flags:    MsgFlags,
    fence_id: Le64, // This value isn't used for anything because we use a future-based interface.
    ctx_id:   Le32, // Unused in 2D mode
    padding:  Le32
}

impl ControlQHeader {
    pub fn new(msg_type: MsgType, flags: MsgFlags) -> Self {
        Self {
            msg_type,
            flags,
            fence_id: u64::to_le(0),
            ctx_id:   u32::to_le(0),
            padding:  u32::to_le(0)
        }
    }
}

// FIXME: Move `ffi_enum!` out of the kernel's `shared` crate and into its own crate outside the
//        kernel, then use it here.
macro_rules! define_msg_type {
    (
        $(
            $(#[$variant_attr:meta])*
            $variant:ident $(= $val:expr)?
        ),* $(,)?
    ) => {
        /// The type of a message sent to or from the GPU.
        #[repr(u32)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum MsgType {
            $(
                $(#[$variant_attr])*
                $variant $(= $val)?
            ),*
        }
        impl core::convert::TryFrom<Le32> for MsgType {
            type Error = InvalidMsgTypeError;

            fn try_from(value: Le32) -> Result<MsgType, Self::Error> {
                match value {
                    $(x if x == MsgType::$variant as Le32 => Ok(MsgType::$variant),)*
                    value => Err(InvalidMsgTypeError::new(value))
                }
            }
        }
        impl From<MsgType> for Le32 {
            fn from(value: MsgType) -> Le32 {
                value as Le32
            }
        }
    };
}

define_msg_type! {
    // 2D commands
    /// Get information about all the scanouts.
    CmdGetDisplayInfo           = u32::to_le(0x0100),
    /// Make a new 2D resource.
    CmdResourceCreate2D         = u32::to_le(0x0101),
    /// Delete a resource.
    CmdResourceUnref            = u32::to_le(0x0102),
    /// Set the parameters for a scanout.
    CmdSetScanout               = u32::to_le(0x0103),
    /// Flush a resource to the screen.
    CmdResourceFlush            = u32::to_le(0x0104),
    /// Transfer data from guest memory to a host resource.
    CmdTransferToHost2D         = u32::to_le(0x0105),
    /// Attach a resource to some backing memory.
    CmdResourceAttachBacking    = u32::to_le(0x0106),
    /// Detach a resource from its backing memory.
    CmdResourceDetachBacking    = u32::to_le(0x0107),
    /// Get the information for a capability set? The specification doesn't document how this works.
    CmdGetCapsetInfo            = u32::to_le(0x0108),
    /// Get the device's capability set? The specification doesn't document how this works.
    CmdGetCapset                = u32::to_le(0x0109),
    /// Get a scanout's VESA EDID blob (if the associated feature has been negotiated).
    CmdGetEdid                  = u32::to_le(0x010a),

    // Cursor commands (best to use the cursor queue for these)
    /// Set the cursor image and position.
    CmdUpdateCursor             = u32::to_le(0x0300),
    /// Set the cursor position (but leave its image unchanged).
    CmdMoveCursor               = u32::to_le(0x0301),

    // Success responses
    /// No data, just success.
    RespOkNoData                = u32::to_le(0x1100),
    /// Information on scanouts.
    RespOkDisplayInfo           = u32::to_le(0x1101),
    /// Information about a capability set? The specification doesn't document how this works.
    RespOkCapsetInfo            = u32::to_le(0x1102),
    /// Specifies a capability set? The specification doesn't document how this works.
    RespOkCapset                = u32::to_le(0x1103),
    /// A scanout's VESA EDID blob.
    RespOkEdid                  = u32::to_le(0x1104),

    // Error responses
    /// An unspecified error.
    RespErrUnspec               = u32::to_le(0x1200),
    /// Unable to complete an operation because something (the host? guest?) ran out of memory.
    RespErrOutOfMemory          = u32::to_le(0x1201),
    /// An error caused by giving the device an invalid scanout ID.
    RespErrInvalidScanoutId     = u32::to_le(0x1202),
    /// An error caused by giving the device an invalid resource ID.
    RespErrInvalidResourceId    = u32::to_le(0x1203),
    /// An error caused by giving the device an invalid context ID.
    RespErrInvalidContextId     = u32::to_le(0x1204),
    /// An error caused by giving the device an invalid parameter.
    RespErrInvalidParameter     = u32::to_le(0x1205)
}

/// An error caused by receiving a message of an unknown type.
pub struct InvalidMsgTypeError {
    message: Le32
}

impl InvalidMsgTypeError {
    const fn new(message: Le32) -> Self {
        Self { message }
    }
}

impl fmt::Display for InvalidMsgTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid GPU message type: {:#x}", u32::from_le(self.message))
    }
}

bitflags! {
    pub struct MsgFlags: Le32 {
        const FENCE = u32::to_le(1); // Forces the device to finish the operation before responding
    }
}

// TODO: This should probably be in a different module.
/// Any error that can occur when interfacing with the GPU.
#[derive(Debug)]
pub enum GpuError {
    /// The device returned a response that was shorter than expected.
    ResponseTooShort(MsgType, usize, usize),
    /// A wrapped error from the virtio crate.
    VirtIoError(VirtIoError),
    /// Allocating a buffer failed.
    AllocError,
    /// The caller passed an invalid parameter to some function.
    InvalidParameter
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::ResponseTooShort(msg_type, actual, expected) =>
                write!(f,
                    "GPU error: command {:#x}: response only {:#x} bytes long, expected at least {:#x} bytes",
                    u32::from_le(Le32::from(msg_type)),
                    actual,
                    expected
                ),
            Self::VirtIoError(ref e) =>
                write!(f, "GPU error: {}", e),
            Self::AllocError =>
                write!(f, "GPU error: failed allocation"),
            Self::InvalidParameter =>
                write!(f, "GPU error: invalid parameter")
        }
    }
}
