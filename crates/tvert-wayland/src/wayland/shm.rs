use rustix::fs::{MemfdFlags, memfd_create};
use std::{fs::File, os::fd::AsFd};
use wayland_client::{
    QueueHandle, WEnum, delegate_noop,
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
};

use crate::{Error, State, wayland::screencopy::ShmRequirements};

#[derive(Debug)]
pub struct ShmBuffer {
    buffer: WlBuffer,
    // Retained for reading pixels after capture completes
    _file: File,
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

        Ok(Self {
            buffer,
            _file: file,
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

delegate_noop!(State: ignore WlShmPool);
delegate_noop!(State: ignore WlBuffer);
