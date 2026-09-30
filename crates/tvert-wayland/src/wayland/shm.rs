use crate::{
    CapturedImage, Error, State,
    wayland::{pixels, screencopy::ShmRequirements},
};
use rustix::fs::{MemfdFlags, memfd_create};
use std::{fs::File, io::Write, os::fd::AsFd};
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
    // Retained for reading pixels after capture completes
    file: File,
}

impl ShmBuffer {
    pub fn new(
        shm: &WlShm,
        advertised_formats: &[Format],
        requirements: ShmRequirements,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, Error> {
        let format = match requirements.format {
            WEnum::Value(
                format
                @ (Format::Argb8888 | Format::Xrgb8888 | Format::Abgr8888 | Format::Xbgr8888),
            ) => format,
            other => return Err(Error::UnsupportedShmFormat(other)),
        };

        if !advertised_formats.contains(&format) {
            return Err(Error::UnsupportedShmFormat(requirements.format));
        }

        if requirements.width == 0 || requirements.height == 0 {
            return Err(Error::InvalidShmLayout);
        }

        let minimum_stride = requirements
            .width
            .checked_mul(4)
            .ok_or(Error::InvalidShmLayout)?;

        if requirements.stride < minimum_stride || !requirements.stride.is_multiple_of(4) {
            return Err(Error::InvalidShmLayout);
        }

        let byte_len = requirements
            .stride
            .checked_mul(requirements.height)
            .ok_or(Error::InvalidShmLayout)?;

        let pool_size = shm_dimensions(byte_len)?;
        let width = shm_dimensions(requirements.width)?;
        let height = shm_dimensions(requirements.height)?;
        let stride = shm_dimensions(requirements.stride)?;

        let fd =
            memfd_create(c"tvert-screenshot", MemfdFlags::CLOEXEC).map_err(std::io::Error::from)?;
        let file = File::from(fd);
        file.set_len(u64::from(byte_len))?;

        let pool = shm.create_pool(file.as_fd(), pool_size, queue_handle, ());
        let buffer = pool.create_buffer(0, width, height, stride, format, queue_handle, ());

        pool.destroy();

        Ok(Self { buffer, file })
    }

    pub const fn proxy(&self) -> &WlBuffer {
        &self.buffer
    }

    pub fn read_image(
        &self,
        requirements: ShmRequirements,
        y_inverted: bool,
    ) -> Result<CapturedImage, Error> {
        pixels::read_image(&self.file, requirements, y_inverted)
    }

    pub fn from_image(
        shm: &WlShm,
        advertised_formats: &[Format],
        image: &CapturedImage,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, Error> {
        let width = image.width.get();
        let height = image.height.get();
        let stride = width.checked_mul(4).ok_or(Error::InvalidShmLayout)?;
        let byte_len = stride.checked_mul(height).ok_or(Error::InvalidShmLayout)?;
        let byte_len = usize::try_from(byte_len).map_err(|_| Error::InvalidShmLayout)?;

        if image.rgba.len() != byte_len {
            return Err(Error::InvalidShmLayout);
        }

        let requirements = ShmRequirements {
            format: WEnum::Value(Format::Argb8888),
            height,
            width,
            stride,
        };

        let mut buffer = Self::new(shm, advertised_formats, requirements, queue_handle)?;

        let mut bytes = Vec::new();
        bytes.try_reserve_exact(byte_len)?;
        bytes.extend_from_slice(&image.rgba);

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
