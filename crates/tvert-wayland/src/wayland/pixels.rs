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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_matches, assert_ok};
    use rstest::rstest;
    use rustix::fs::{MemfdFlags, memfd_create};
    use std::io::{ErrorKind, Write};

    fn read_fixture(
        bytes: &[u8],
        requirements: ShmRequirements,
        y_inverted: bool,
    ) -> Result<CapturedImage, Error> {
        let fd =
            memfd_create(c"tvert-pixel-test", MemfdFlags::CLOEXEC).map_err(std::io::Error::from)?;

        let mut file = File::from(fd);
        file.write_all(bytes)?;

        read_image(&file, requirements, y_inverted)
    }

    #[rstest]
    #[case(Format::Argb8888, [30, 20, 10, 255])]
    #[case(Format::Xrgb8888, [30, 20, 10, 0])]
    #[case(Format::Abgr8888, [10, 20, 30, 255])]
    #[case(Format::Xbgr8888, [10, 20, 30, 0])]
    fn converts_supported_channel_orders(#[case] format: Format, #[case] bytes: [u8; 4]) {
        let image = assert_ok!(read_fixture(
            &bytes,
            ShmRequirements {
                format: WEnum::Value(format),
                width: 1,
                height: 1,
                stride: 4,
            },
            false
        ));
        assert_eq!(image.rgba, [10, 20, 30, 255], "{format:?}");
    }

    #[test]
    fn converts_premultiplied_alpha() {
        let image = assert_ok!(read_fixture(
            &[0, 32, 64, 128],
            ShmRequirements {
                format: WEnum::Value(Format::Argb8888),
                width: 1,
                height: 1,
                stride: 4,
            },
            false,
        ));
        assert_eq!(image.rgba, [128, 64, 0, 128]);
    }

    #[test]
    fn handles_fully_transparent_pixels() {
        let image = assert_ok!(read_fixture(
            &[0, 0, 0, 0],
            ShmRequirements {
                format: WEnum::Value(Format::Argb8888),
                width: 1,
                height: 1,
                stride: 4,
            },
            false
        ));
        assert_eq!(image.rgba, [0, 0, 0, 0]);
    }

    #[rstest]
    #[case(false, [255, 0, 0, 255, 0, 0, 255, 255])]
    #[case(true, [0, 0, 255, 255, 255, 0, 0, 255])]
    fn skips_padding_and_normalizes_row_order(#[case] y_inverted: bool, #[case] expected: [u8; 8]) {
        let bytes = [0, 0, 255, 0, 99, 98, 97, 96, 255, 0, 0, 0, 95, 94, 93, 92];

        let image = assert_ok!(read_fixture(
            &bytes,
            ShmRequirements {
                format: WEnum::Value(Format::Xrgb8888),
                width: 1,
                height: 2,
                stride: 8,
            },
            y_inverted,
        ));

        assert_eq!(image.width.get(), 1);
        assert_eq!(image.height.get(), 2);
        assert_eq!(image.rgba, expected, "y_inverted={y_inverted}");
    }

    #[rstest]
    #[case(0, 1)]
    #[case(1, 0)]
    fn rejects_zero_dimensions(#[case] width: u32, #[case] height: u32) {
        let error = assert_err!(read_fixture(
            &[],
            ShmRequirements {
                format: WEnum::Value(Format::Xrgb8888),
                width,
                height,
                stride: 4,
            },
            false,
        ));
        assert_matches!(error, Error::InvalidShmLayout);
    }

    #[test]
    fn rejects_stride_smaller_than_a_row() {
        let error = assert_err!(read_fixture(
            &[],
            ShmRequirements {
                format: WEnum::Value(Format::Xrgb8888),
                width: 2,
                height: 1,
                stride: 4,
            },
            false,
        ));

        assert_matches!(error, Error::InvalidShmLayout);
    }

    #[test]
    fn reports_truncated_backing_storage() {
        let result = assert_err!(read_fixture(
            &[0, 0, 0, 255],
            ShmRequirements {
                format: WEnum::Value(Format::Xrgb8888),
                width: 1,
                height: 2,
                stride: 4,
            },
            false,
        ));

        assert_matches!(
            result,
            Error::ShmIo(error) if error.kind() == ErrorKind::UnexpectedEof
        );
    }
}
