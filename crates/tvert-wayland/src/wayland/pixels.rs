use crate::{
    Error,
    wayland::shm::{ShmFormat, ShmLayout},
};
use std::{fs::File, os::unix::fs::FileExt};
use tvert_image::CapturedImage;

pub fn read_image(
    file: &File,
    layout: &ShmLayout,
    y_inverted: bool,
) -> Result<CapturedImage, Error> {
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(layout.rgba_len())?;
    rgba.resize(layout.rgba_len(), 0);

    for (mut source_row, row) in
        (0..layout.height().get()).zip(rgba.chunks_exact_mut(layout.row_bytes()))
    {
        if y_inverted {
            source_row = layout
                .height()
                .get()
                .checked_sub(1)
                .and_then(|last| last.checked_sub(source_row))
                .ok_or(Error::InvalidShmLayout)?;
        }

        file.read_exact_at(row, layout.row_offset(source_row)?)?;

        for pixel in row.as_chunks_mut::<4>().0 {
            *pixel = rgba_pixel(*pixel, layout.format())?;
        }
    }

    CapturedImage::new(layout.size(), rgba).map_err(Error::from)
}

fn rgba_pixel([first, green, last, alpha]: [u8; 4], format: ShmFormat) -> Result<[u8; 4], Error> {
    let (red, blue, alpha) = match format {
        ShmFormat::Argb8888 => (last, first, alpha),
        ShmFormat::Xrgb8888 => (last, first, 255),
        ShmFormat::Abgr8888 => (first, last, alpha),
        ShmFormat::Xbgr8888 => (first, last, 255),
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
    use crate::wayland::shm::ShmRequirements;
    use claims::{assert_err, assert_ok};
    use rstest::rstest;
    use rustix::fs::{MemfdFlags, memfd_create};
    use std::{
        assert_matches,
        io::{ErrorKind, Write},
    };
    use wayland_client::{WEnum, protocol::wl_shm::Format};

    fn read_fixture(
        bytes: &[u8],
        requirements: ShmRequirements,
        y_inverted: bool,
    ) -> Result<CapturedImage, Error> {
        let layout = ShmLayout::try_from(requirements)?;
        let fd =
            memfd_create(c"tvert-pixel-test", MemfdFlags::CLOEXEC).map_err(std::io::Error::from)?;

        let mut file = File::from(fd);
        file.write_all(bytes)?;

        read_image(&file, &layout, y_inverted)
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
        assert_eq!(image.rgba(), [10, 20, 30, 255], "{format:?}");
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
        assert_eq!(image.rgba(), [128, 64, 0, 128]);
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
        assert_eq!(image.rgba(), [0, 0, 0, 0]);
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

        let size = image.size();
        assert_eq!(size.width().get(), 1);
        assert_eq!(size.height().get(), 2);
        assert_eq!(image.rgba(), expected, "y_inverted={y_inverted}");
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
