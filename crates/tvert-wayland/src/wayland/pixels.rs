use crate::{Error, wayland::screencopy::ShmRequirements};
use std::{fs::File, num::NonZeroU32, os::unix::fs::FileExt};
use wayland_client::{WEnum, protocol::wl_shm::Format};

/// An owned image with tightly packed, top-to-bottom RGBA8 pixels.
///
/// Alpha is straight, not premultiplied.
#[derive(Debug)]
pub struct CapturedImage {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    /// Consecutive red, green, blue, and alpha bytes, without row padding
    pub rgba: Vec<u8>,
}

pub fn read_image(
    file: &File,
    requirements: ShmRequirements,
    y_inverted: bool,
) -> Result<CapturedImage, Error> {
    let format = match requirements.format {
        WEnum::Value(
            format @ (Format::Argb8888 | Format::Xrgb8888 | Format::Abgr8888 | Format::Xbgr8888),
        ) => format,
        other => return Err(Error::UnsupportedShmFormat(other)),
    };

    let width = NonZeroU32::new(requirements.width).ok_or(Error::InvalidShmLayout)?;
    let height = NonZeroU32::new(requirements.height).ok_or(Error::InvalidShmLayout)?;

    let row_bytes = width.get().checked_mul(4).ok_or(Error::InvalidShmLayout)?;

    if requirements.stride < row_bytes {
        return Err(Error::InvalidShmLayout);
    }

    let row_bytes = usize::try_from(row_bytes).map_err(|_| Error::InvalidShmLayout)?;
    let row_count = usize::try_from(height.get()).map_err(|_| Error::InvalidShmLayout)?;
    let byte_len = row_bytes
        .checked_mul(row_count)
        .ok_or(Error::InvalidShmLayout)?;

    let mut rgba = Vec::new();
    rgba.try_reserve_exact(byte_len)?;
    rgba.resize(byte_len, 0);

    for (mut source_row, row) in (0..height.get()).zip(rgba.chunks_exact_mut(row_bytes)) {
        if y_inverted {
            source_row = height
                .get()
                .checked_sub(1)
                .and_then(|last| last.checked_sub(source_row))
                .ok_or(Error::InvalidShmLayout)?;
        }

        let offset = u64::from(source_row)
            .checked_mul(u64::from(requirements.stride))
            .ok_or(Error::InvalidShmLayout)?;

        file.read_exact_at(row, offset)?;

        for pixel in row.as_chunks_mut::<4>().0 {
            let converted = rgba_pixel(pixel, format)?;
            pixel.copy_from_slice(&converted);
        }
    }

    Ok(CapturedImage {
        width,
        height,
        rgba,
    })
}

fn rgba_pixel(pixel: &[u8], format: Format) -> Result<[u8; 4], Error> {
    let &[first, green, last, alpha] = pixel else {
        return Err(Error::InvalidShmLayout);
    };

    let (red, blue, alpha) = match format {
        Format::Argb8888 => (last, first, alpha),
        Format::Xrgb8888 => (last, first, 255),
        Format::Abgr8888 => (first, last, alpha),
        Format::Xbgr8888 => (first, last, 255),
        other => return Err(Error::UnsupportedShmFormat(WEnum::Value(other))),
    };

    if alpha == 255 {
        return Ok([red, green, blue, alpha]);
    }

    Ok([
        unpremultiply(red, alpha)?,
        unpremultiply(green, alpha)?,
        unpremultiply(blue, alpha)?,
        alpha,
    ])
}

fn unpremultiply(channel: u8, alpha: u8) -> Result<u8, Error> {
    if alpha == 0 {
        return Ok(0);
    }

    let alpha = u32::from(alpha);
    let rounding = alpha.checked_div(2).ok_or(Error::InvalidShmLayout)?;

    let channel = u32::from(channel)
        .checked_mul(255)
        .and_then(|value| value.checked_add(rounding))
        .and_then(|value| value.checked_div(alpha))
        .ok_or(Error::InvalidShmLayout)?;

    u8::try_from(channel.min(255)).map_err(|_| Error::InvalidShmLayout)
}
