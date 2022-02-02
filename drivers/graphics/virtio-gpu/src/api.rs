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
//! specified in *TODO: INSERT NAME OF FILE THAT DEFINES THE INTERFACE*.

use {
    alloc::{
        alloc::AllocError,
        vec::Vec
    },
    core::{
        iter::Rev,
        mem,
        num::NonZeroU32,
        ops::Range,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering}
    },
    bitflags::bitflags,
    libphoenix::allocator::{Allocator, PhysBox},
    virtio::virtqueue::{
        VirtQueue,
        future::Executor
    },
    crate::msg::*
};

/// This type represents a tile in an image. It's really just a number, and the driver has no idea
/// what it means. In a text mode, for instance, this would be the numeric representation of a
/// character, most likely in ASCII. For any image understood by the VirtIO GPU there is only one
/// possible tile (a solid-colored pixel), so every bit is ignored when writing, and zero is
/// returned when reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Tile(pub u32);

/// This type represents a color in an image. This is either paletted or truecolor, and in the
/// latter case it can be stored in any format (as long as it's not bigger than this type). The
/// driver doesn't really care what the format is; it just copies the color bitwise into an image.
/// The graphics library is responsible for producing colors in the correct format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Color(pub u32);

#[derive(Debug)]
pub struct DisplayInfo {
    pub rect:  Rectangle,
    pub flags: DisplayInfoFlags
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

/// Returns `Ok` if the image is accepted as the display's framebuffer and `Err` if it's
/// incompatible.
pub async fn set_display_framebuffer(
        virtq: &VirtQueue<'_>,
        display_idx: u32,
        fb: &Image<'_, '_>
) -> Result<(), GpuError> {
    if let Some(ref virtq_info) = fb.virtq_info {
        send_recv_cmd(
            CmdSetScanout::new(display_idx, virtq_info.resource_id.get(), fb.rect(), MsgFlags::FENCE)
                .map_err(|AllocError| GpuError::AllocError)?,
            virtq
        ).await?;
        Ok(()) // TODO: Return some error information on failure.
    } else {
        // The image couldn't be given to the GPU when it was created, probably because of its
        // format.
        Err(GpuError::InvalidParameter)
    }
}

#[derive(Debug)]
pub struct Image<'a, 'b: 'a> {
    tile_length:     u8, // Bytes per tile (for specifying which tile, not the colors)
    colors_per_tile: u8,
    bits_per_color:  u8,
    bytes_per_color: u8, // Equal to bits_per_color / 8, rounded up

    width:           u32, // Measured in tiles
    // Height can be calculated as needed.

    virtq_info:      Option<ImageVirtqInfo<'a, 'b>>, // `Some` if the GPU has a copy of this image, else `None`
    backing:         PhysBox<[u8]>,

    drawing_colors:  Vec<Color> // The colors that will be used to draw new tiles
}

#[derive(Debug)]
struct ImageVirtqInfo<'a, 'b: 'a> {
    virtq:       &'a VirtQueue<'b>,
    resource_id: NonZeroU32
}

impl<'a, 'b: 'a> Image<'a, 'b> {
    /// Returns `Ok` with a image containing the given settings if they work with the hardware.
    /// Otherwise returns `Err`. Also provides the image to the GPU as a 2D resource if
    /// `as_gpu_resource` is `true`, allowing it to, e.g., be used as a framebuffer.
    pub async fn new(
            virtq: Option<&'a VirtQueue<'b>>,
            tile_length: u8,
            colors_per_tile: u8,
            bits_per_color: u8,
            width: u32,
            height: u32
    ) -> Result<Image<'a, 'b>, GpuError> {
        // TODO: Instead of doing this (insufficient) check at the beginning, catch any unwraps
        // that happen and return `Err(())` in that case. Otherwise, we can't
        // catch a failure in `Vec::with_capacity` or `slice::repeat`. Unfortunately, we can't do
        // this when panics are set to "abort", so we'll need to get unwrapping working.
        if width > 4096 || height > 4096 // Larger screens can exist, but they should have dedicated VRAM.
                || tile_length as usize > mem::size_of::<Tile>()
                || bits_per_color as usize > mem::size_of::<Color>() * 8 {
            return Err(GpuError::InvalidParameter);
        }

        let bytes_per_color = (bits_per_color + 7) / 8;
        let backing_len = (
            (tile_length as u32 + colors_per_tile as u32 * bytes_per_color as u32) * width * height
        ) as usize;

        let mut backing = Allocator.malloc_phys_bytes(backing_len, 1, mem::size_of::<*mut u8>() * 8)
            .map_err(|AllocError| GpuError::AllocError)?;
        for byte in backing.iter_mut() {
            *byte = 0;
        }

        let mut img = Image {
            tile_length,
            colors_per_tile,
            bits_per_color,
            bytes_per_color,
            width,
            virtq_info: None,
            backing,
            drawing_colors: Vec::with_capacity(colors_per_tile as usize)
        };
        img.drawing_colors.resize_with(colors_per_tile as usize, Default::default);

        if let Some(virtq) = virtq {
            if img.virtio_compatible() {
                if let Some(resource_id) = alloc_resource() {
                    img.virtq_info = Some(ImageVirtqInfo { virtq, resource_id });
                }
            }
        }

        if let Some(ref virtq_info) = img.virtq_info {
            send_recv_cmd(
                CmdResourceCreate2D::new(
                    virtq_info.resource_id.get(),
                    Resource2DFormat::BytesRGBA,
                    width,
                    height,
                    MsgFlags::FENCE
                )
                    .map_err(|AllocError| GpuError::AllocError)?,
                virtq_info.virtq
            ).await?;
            send_recv_cmd(
                CmdResourceAttachBacking::new(virtq_info.resource_id.get(), &[&img.backing], MsgFlags::FENCE)
                    .map_err(|AllocError| GpuError::AllocError)?,
                virtq_info.virtq
            ).await?;
            Ok(img)
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
    pub async fn send_rect(&self, rect: Rectangle) -> Result<(), GpuError> {
        if let Some(ref virtq_info) = self.virtq_info {
            let bytes_per_tile = self.tile_length as u64 + self.colors_per_tile as u64 * self.bytes_per_color as u64;
            let dest = (rect.x + rect.y * self.width) as u64 * bytes_per_tile;
            send_recv_cmd(
                CmdTransferToHost2D::new(
                    virtq_info.resource_id.get(),
                    rect,
                    dest,
                    MsgFlags::FENCE
                )
                    .map_err(|AllocError| GpuError::AllocError)?,
                virtq_info.virtq
            ).await?;
        }
        Ok(())
    }

    /// Equivalent to `self.send_rect(self.rect())`.
    pub async fn send_all(&self) -> Result<(), GpuError> {
        self.send_rect(self.rect()).await
    }

    /// Flushes the given part of the image to the screen. Note that this has an effect only if the
    /// image is the backing for a scanout.
    pub async fn flush_rect(&self, rect: Rectangle) -> Result<(), GpuError> {
        if let Some(ref virtq_info) = self.virtq_info {
            send_recv_cmd(
                CmdResourceFlush::new(virtq_info.resource_id.get(), rect, MsgFlags::FENCE)
                    .map_err(|AllocError| GpuError::AllocError)?,
                virtq_info.virtq
            ).await?;
        }
        Ok(())
    }

    /// Equivalent to `self.flush_rect(self.rect())`.
    pub async fn flush_all(&self) -> Result<(), GpuError> {
        self.flush_rect(self.rect()).await
    }

    /// Returns a rectangle representing the image's position and dimensions.
    pub fn rect(&self) -> Rectangle {
        Rectangle::new(0, 0, self.width(), self.height())
    }

    /// Returns the image's width, in tiles.
    pub fn width(&self) -> u32 { self.width }

    /// Returns the image's height, in tiles.
    pub fn height(&self) -> u32 {
        let bytes_per_tile = self.tile_length() as u32 + self.colors_per_tile() as u32 * self.bytes_per_color as u32;
        let bytes_per_row = bytes_per_tile * self.width();
        self.backing.len() as u32 / bytes_per_row
    }

    /// Returns the number of bytes used to select each tile.
    pub fn tile_length(&self) -> u8 { self.tile_length }

    /// Returns the number of colors that are in each tile.
    pub fn colors_per_tile(&self) -> u8 { self.colors_per_tile }

    /// Returns the number of bits used to represent a color.
    pub fn bits_per_color(&self) -> u8 { self.bits_per_color }

    /// Sets a range of colors to be used when drawing new tiles to this image (intended to be used
    /// when setting all colors at the same time).
    pub fn set_colors(&mut self, first_index: usize, colors: &[Color]) {
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
    pub fn colors(&self) -> &[Color] { &*self.drawing_colors }

    /// Returns the tile found at the given coordinates.
    pub fn tile(&self, x: u32, y: u32) -> Option<(Tile, Vec<Color>)> {
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
    pub fn draw_rect_region(&mut self, mut rect: Rectangle, tile: Tile) {
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

    /// Blits the given tiles to the given rectangle. Like the C standard library's `memcpy`,
    /// this function isn't safe for copying between overlapping parts of the same image. Use
    /// `move_rect` for that.
    ///
    /// # Returns
    /// `Ok` if the blit succeeded. `Err` if not, probably because the images use different formats
    /// (tileset size, color depth, and number of colors per tile).
    pub fn blit(&mut self, src: &Image, src_rect: Rectangle, dest_x: u32, dest_y: u32) -> Result<(), ()> {
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
    pub fn move_rect(&mut self, src_rect: Rectangle, dest_x: u32, dest_y: u32) {
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

impl<'a, 'b: 'a> Drop for Image<'a, 'b> {
    fn drop(&mut self) {
        if let Some(ref virtq_info) = self.virtq_info {
            Executor::new()
                .spawn(async move {
                    match CmdResourceUnref::new(virtq_info.resource_id.get()) {
                        Err(AllocError) => {}, // TODO: Log an error, but don't panic. This is just a memory leak.
                        Ok(cmd) => {
                            if let Err(e) = send_recv_cmd(cmd, virtq_info.virtq).await {
                                // TODO: Log an error, but don't panic. This is just a memory leak.
                            }
                            free_resource(virtq_info.resource_id);
                        }
                    }
                })
                .block_on_all();
        }
    }
}

const MAX_RESOURCES: usize = 4096; // Must be a power of 2 and fit in a `u32`
static RESOURCE_IDS_USED: [AtomicBool; MAX_RESOURCES] = [const { AtomicBool::new(false) }; 4096];

// Allocates an ID for a new Resource2D to be given to the GPU.
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

// Frees a Resource2D ID so it can be reused. (This doesn't communicate with the GPU.)
// Panics if that ID wasn't already allocated.
fn free_resource(id: NonZeroU32) {
    let i = id.get() - 1;
    assert!(RESOURCE_IDS_USED[i as usize].swap(false, Ordering::AcqRel));
}
