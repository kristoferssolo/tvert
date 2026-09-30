mod layout;
mod requirements;

pub use layout::{ShmLayout, Stride};
pub use requirements::ShmRequirements;

use crate::{Error, state::State, wayland::pixels};
use rustix::fs::{MemfdFlags, memfd_create};
use std::{fs::File, io::Write, os::fd::AsFd};
use tvert_image::CapturedImage;
use wayland_client::{
    QueueHandle, WEnum, delegate_noop,
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
};

#[derive(Debug)]
pub struct ShmBuffer {
    buffer: WlBuffer,
    file: File,
    layout: ShmLayout,
}

impl ShmBuffer {
    /// Allocates a SHM buffer from compositor-advertised requirements.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedShmFormat`] if the format is unsupported
    /// or was not advertised by this connection's SHM global.
    ///
    /// Returns [`Error::InvalidShmLayout`] if the layout cannot be used,
    /// or [`Error::ShmIo`] if creating or sizing the backing file fails.
    pub fn new(
        shm: &WlShm,
        advertised_formats: &[Format],
        requirements: ShmRequirements,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, Error> {
        let layout = ShmLayout::try_from(requirements)?;
        Self::from_layout(shm, advertised_formats, layout, queue_handle)
    }

    pub const fn proxy(&self) -> &WlBuffer {
        &self.buffer
    }

    /// Reads pixels using the layout that created this buffer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ShmIo`] if reading fails, [`Error::ImageAllocation`]
    /// if allocating image storage fails, or [`Error::InvalidShmLayout`]
    /// if row addressing or pixel conversion fails.
    ///
    /// Returns [`Error::Image`] if constructing the captured image fails.
    pub fn read_image(&self, y_inverted: bool) -> Result<CapturedImage, Error> {
        pixels::read_image(&self.file, &self.layout, y_inverted)
    }

    /// Creates a display buffer from a tightly packed RGBA8 image.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidShmLayout`] if the dimensions cannot be
    /// represented by the SHM protocol, , [`Error::UnsupportedShmFormat`]
    /// if ARGB8888 is unavailable, [`Error::ImageAllocation`] if temporary
    /// storage allocation fails, or [`Error::ShmIo`] if creating or
    /// writing the backing file fails.
    pub fn from_image(
        shm: &WlShm,
        advertised_formats: &[Format],
        image: &CapturedImage,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, Error> {
        let stride = Stride::packed(image.size().width())?;
        let size = image.size();

        let layout = ShmLayout::new(
            WEnum::Value(Format::Argb8888),
            size.width().get(),
            size.height().get(),
            stride.bytes(),
        )?;

        let byte_len = layout.rgba_len();

        if image.rgba().len() != byte_len {
            return Err(Error::InvalidShmLayout);
        }

        let mut buffer = Self::from_layout(shm, advertised_formats, layout, queue_handle)?;

        let mut bytes = Vec::new();
        bytes.try_reserve_exact(byte_len)?;
        bytes.extend_from_slice(image.rgba());

        for pixel in bytes.as_chunks_mut::<4>().0 {
            let [red, green, blue, alpha] = *pixel;

            *pixel = [
                premultiply(blue, alpha)?,
                premultiply(green, alpha)?,
                premultiply(red, alpha)?,
                alpha,
            ];
        }

        buffer.file.write_all(&bytes)?;

        Ok(buffer)
    }

    fn from_layout(
        shm: &WlShm,
        advertised_formats: &[Format],
        layout: ShmLayout,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, Error> {
        if !advertised_formats.contains(&layout.format()) {
            return Err(Error::UnsupportedShmFormat(WEnum::Value(layout.format())));
        }

        let width = shm_dimensions(layout.width().get())?;
        let height = shm_dimensions(layout.height().get())?;
        let stride = shm_dimensions(layout.stride().bytes())?;

        let fd =
            memfd_create(c"tvert-screenshot", MemfdFlags::CLOEXEC).map_err(std::io::Error::from)?;
        let file = File::from(fd);
        file.set_len(layout.file_len())?;

        let pool = shm.create_pool(file.as_fd(), layout.pool_size(), queue_handle, ());
        let buffer =
            pool.create_buffer(0, width, height, stride, layout.format(), queue_handle, ());

        pool.destroy();

        Ok(Self {
            buffer,
            file,
            layout,
        })
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

fn shm_dimensions(value: u32) -> Result<i32, Error> {
    i32::try_from(value).map_err(|_| Error::InvalidShmLayout)
}

fn premultiply(channel: u8, alpha: u8) -> Result<u8, Error> {
    let value = u32::from(channel)
        .checked_mul(u32::from(alpha))
        .and_then(|value| value.checked_add(127))
        .and_then(|value| value.checked_div(255))
        .ok_or(Error::InvalidShmLayout)?;

    u8::try_from(value).map_err(|_| Error::InvalidShmLayout)
}

delegate_noop!(State: ignore WlShmPool);
delegate_noop!(State: ignore WlBuffer);
