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

//! This module implements the driver's interface with other programs. The actual interface is
//! specified in *INSERT NAME OF FILE THAT DEFINES THE INTERFACE*.

use {
    alloc::{
        alloc::AllocError,
        vec::Vec
    },
    core::mem,
    bitflags::bitflags,
    virtio::virtqueue::VirtQueue,
    crate::msg::*
};

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
    pub async fn all(virtq: &VirtQueue<'_>) -> Result<Vec<DisplayInfo>, GpuError> {
        let mut response = send_recv_cmd(
            CmdGetDisplayInfo::new()
                .map_err(|AllocError| GpuError::AllocError)?,
            virtq
        ).await?;
        let valid_data_bytes = response.valid_bytes().saturating_sub(unsafe {
            (&response.buffer().response.displays as *const _ as *const u8)
                .offset_from(&**response.buffer() as *const CmdGetDisplayInfo as *const u8) as usize
        });
        let raw = &mut response.buffer_mut().response.displays[0 .. valid_data_bytes / mem::size_of::<RawDisplayInfo>()];
        let mut displays = Vec::with_capacity(raw.len());
        for raw_display in raw {
            let flags = if u32::from_le(raw_display.enabled) != 0 {
                DisplayInfoFlags::ENABLED
            } else {
                DisplayInfoFlags::empty()
            };

            displays.push(DisplayInfo {
                rect: Rectangle::from(mem::replace(&mut raw_display.rect, Default::default())),
                flags
            });
        }
        Ok(displays)
    }

    pub async fn one(virtq: &VirtQueue<'_>, idx: usize) -> Result<DisplayInfo, GpuError> {
        match Self::all(virtq).await {
            Ok(mut all) if all.len() > idx => Ok(all.swap_remove(idx)),
            Err(e)                         => Err(e),
            _                              => Err(GpuError::InvalidParameter)
        }
    }
}

bitflags! {
    pub struct DisplayInfoFlags: u32 {
        const ENABLED = 0x1;
    }
}
