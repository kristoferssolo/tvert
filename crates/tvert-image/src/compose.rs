use image::{RgbaImage, buffer::ConvertBuffer, imageops};
use std::ops::Range;

use crate::{
    CapturedImage, Error, PixelSize,
    premultiplied::{premultiplied_region, unpremultiply},
    region::RgbaRegion,
};

pub fn paint(
    rgba: &mut [u8],
    size: PixelSize,
    source: &CapturedImage,
    x: Range<usize>,
    y: Range<usize>,
) -> Result<(), Error> {
    let region = RgbaRegion::new(size, x, y)?.ok_or(Error::InvalidPaintBounds)?;

    if source.size() == region.size() {
        return replace(rgba, &region, source.rgba());
    }

    let resampled = resample(source, region.size())?;
    replace(rgba, &region, resampled.as_raw())
}

fn replace(rgba: &mut [u8], region: &RgbaRegion, pixels: &[u8]) -> Result<(), Error> {
    let row_bytes = region.size().row_byes()?;

    for (row, source) in region.rows_mut(rgba).zip(pixels.chunks_exact(row_bytes)) {
        row?.copy_from_slice(source);
    }

    Ok(())
}

fn resample(image: &CapturedImage, size: PixelSize) -> Result<RgbaImage, Error> {
    let (width, height) = image.size().try_into()?;
    let whole = RgbaRegion::new(image.size(), 0..width, 0..height)?.ok_or(Error::SizeOverflow)?;
    let source = premultiplied_region(image.rgba(), &whole)?;

    let (width, height) = size.into();
    let mut scaled = imageops::resize(&source, width, height, imageops::FilterType::Triangle);
    unpremultiply(&mut scaled);

    Ok(scaled.convert())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const CLEAR: [u8; 4] = [0; 4];

    fn image(width: u32, height: u32, pixels: &[[u8; 4]]) -> CapturedImage {
        let size = assert_ok!(PixelSize::try_from((width, height)));
        assert_ok!(CapturedImage::new(size, pixels.concat()))
    }

    fn transparent(width: u32, height: u32) -> CapturedImage {
        let size = assert_ok!(PixelSize::try_from((width, height)));
        assert_ok!(CapturedImage::transparent(size))
    }

    #[test]
    fn paints_scaled_and_exact_sources_without_touching_other_pixels() {
        let mut canvas = transparent(3, 3);

        assert_ok!(canvas.paint(&image(1, 1, &[RED]), 0..2, 0..2));
        assert_ok!(canvas.paint(&image(1, 1, &[BLUE]), 2..3, 0..1));

        let expected = [RED, RED, BLUE, RED, RED, CLEAR, CLEAR, CLEAR, CLEAR];
        assert_eq!(canvas.rgba(), expected.concat());
    }

    #[test]
    fn resampling_does_not_tint_edges_with_transparent_colors() {
        let mut canvas = transparent(4, 1);

        assert_ok!(canvas.paint(&image(2, 1, &[RED, [0, 0, 255, 0]]), 0..4, 0..1));

        let pixels = canvas.rgba().as_chunks::<4>().0;
        assert!(pixels.iter().any(|&[.., alpha]| alpha > 0 && alpha < 255));
        for &[red, green, blue, alpha] in pixels {
            if alpha > 0 {
                assert_eq!([red, green, blue], [255, 0, 0]);
            }
        }
    }
}
