use crate::{
    Error, PixelSize,
    color::{average_color, weighted_channels},
    region::RgbaRegion,
};
use std::{num::NonZeroU32, ops::Range};

pub fn pixelate(
    rgba: &mut [u8],
    size: PixelSize,
    x: Range<usize>,
    y: Range<usize>,
    block_size: NonZeroU32,
) -> Result<(), Error> {
    RgbaRegion::new(size, x.clone(), y.clone())?.ok_or(Error::InvalidPixelationBounds)?;
    let block_size = usize::try_from(block_size.get()).map_err(|_| Error::SizeOverflow)?;
    for top in y.clone().step_by(block_size) {
        let rows = top..top.saturating_add(block_size).min(y.end);

        for left in x.clone().step_by(block_size) {
            let right = left.saturating_add(block_size).min(x.end);
            let region = RgbaRegion::new(size, left..right, rows.clone())?
                .ok_or(Error::InvalidPixelationBounds)?;
            let color = block_color(rgba, &region)?;

            for row in region.rows_mut(rgba) {
                for pixel in row?.as_chunks_mut::<4>().0 {
                    *pixel = color;
                }
            }
        }
    }

    Ok(())
}

fn block_color(rgba: &[u8], region: &RgbaRegion) -> Result<[u8; 4], Error> {
    let mut sums = [0u128; 4];
    let mut count = 0u128;
    for row in region.rows(rgba) {
        for pixel in row?.as_chunks::<4>().0 {
            for (sum, channel) in sums.iter_mut().zip(weighted_channels(*pixel)?) {
                *sum = sum
                    .checked_add(u128::from(channel))
                    .ok_or(Error::SizeOverflow)?;
            }
            count = count.checked_add(1).ok_or(Error::SizeOverflow)?;
        }
    }
    average_color(sums, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapturedImage;
    use claims::{assert_err, assert_ok, assert_some};
    use std::assert_matches;

    fn image(width: u32, height: u32, rgba: Vec<u8>) -> CapturedImage {
        let size = assert_ok!(PixelSize::try_from((width, height)));
        assert_ok!(CapturedImage::new(size, rgba))
    }

    #[test]
    fn clips_edge_blocks_and_preserves_pixels_outside_the_region() {
        let rgba = (0..15).flat_map(|red| [red, 20, 30, 255]).collect();
        let mut image = image(5, 3, rgba);
        let block_size = assert_some!(NonZeroU32::new(3));

        assert_ok!(image.pixelate(1..5, 1..3, block_size));

        let expected = [0, 1, 2, 3, 4, 5, 9, 9, 9, 11, 10, 9, 9, 9, 11]
            .into_iter()
            .flat_map(|red| [red, 20, 30, 255])
            .collect::<Vec<_>>();
        assert_eq!(image.rgba(), expected);
    }

    #[test]
    fn transparent_pixels_do_not_tint_the_average() {
        let mut image = image(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 0]);
        assert_ok!(image.pixelate(0..2, 0..1, NonZeroU32::MAX));
        assert_eq!(image.rgba(), [255, 0, 0, 127, 255, 0, 0, 127]);
    }

    #[test]
    fn fully_transparent_blocks_remain_transparent() {
        let mut image = image(2, 1, vec![255, 0, 0, 0, 0, 0, 255, 0]);
        assert_ok!(image.pixelate(0..2, 0..1, NonZeroU32::MAX));
        assert_eq!(image.rgba(), [0; 8]);
    }

    #[test]
    fn rejects_invalid_bounds_without_changing_pixels() {
        let original = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let mut image = image(2, 1, original.clone());

        let error = assert_err!(image.pixelate(0..3, 0..1, NonZeroU32::MAX));

        assert_matches!(error, Error::InvalidPixelationBounds);
        assert_eq!(image.rgba(), original);
    }
}
