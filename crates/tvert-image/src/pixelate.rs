use crate::{Error, PixelSize};
use std::{num::NonZeroU32, ops::Range};

pub fn pixelate(
    rgba: &mut [u8],
    size: PixelSize,
    x: Range<usize>,
    y: Range<usize>,
    block_size: NonZeroU32,
) -> Result<(), Error> {
    let width = usize::try_from(size.width().get()).map_err(|_| Error::SizeOverflow)?;
    let height = usize::try_from(size.height().get()).map_err(|_| Error::SizeOverflow)?;

    if x.is_empty() || y.is_empty() || x.end > width || y.end > height {
        return Err(Error::InvalidPixelationBounds);
    }

    let row_bytes = width.checked_mul(4).ok_or(Error::SizeOverflow)?;
    let block_size = usize::try_from(block_size.get()).map_err(|_| Error::SizeOverflow)?;

    for top in y.clone().step_by(block_size) {
        let rows = top..top.saturating_add(block_size).min(y.end);

        for left in x.clone().step_by(block_size) {
            let right = left.saturating_add(block_size).min(x.end);
            let start = left.checked_mul(4).ok_or(Error::SizeOverflow)?;
            let end = right.checked_mul(4).ok_or(Error::SizeOverflow)?;
            let columns = start..end;
            let color = average_color(rgba, row_bytes, &columns, &rows)?;
            fill_block(rgba, row_bytes, &columns, &rows, color)?;
        }
    }

    Ok(())
}

fn average_color(
    rgba: &[u8],
    row_bytes: usize,
    columns: &Range<usize>,
    rows: &Range<usize>,
) -> Result<[u8; 4], Error> {
    let mut sums = [0u128; 4];
    let mut count = 0u128;

    for row in rgba
        .chunks_exact(row_bytes)
        .skip(rows.start)
        .take(rows.len())
    {
        let selected = row.get(columns.clone()).ok_or(Error::SizeOverflow)?;

        for [red, green, blue, alpha] in selected.as_chunks::<4>().0 {
            let alpha = u128::from(*alpha);
            let channels = [u128::from(*red), u128::from(*green), u128::from(*blue), 1];

            for (sum, channel) in sums.iter_mut().zip(channels) {
                let weighted = channel.checked_mul(alpha).ok_or(Error::SizeOverflow)?;
                *sum = sum.checked_add(weighted).ok_or(Error::SizeOverflow)?;
            }

            count = count.checked_add(1).ok_or(Error::SizeOverflow)?;
        }
    }

    let [red, green, blue, alpha] = sums;
    if alpha == 0 {
        return Ok([0; 4]);
    }

    let mut color = [0; 4];
    for (channel, (sum, divisor)) in
        color
            .iter_mut()
            .zip([(red, alpha), (green, alpha), (blue, alpha), (alpha, count)])
    {
        let average = sum.checked_div(divisor).ok_or(Error::SizeOverflow)?;
        *channel = u8::try_from(average).map_err(|_| Error::SizeOverflow)?;
    }

    Ok(color)
}

fn fill_block(
    rgba: &mut [u8],
    row_bytes: usize,
    columns: &Range<usize>,
    rows: &Range<usize>,
    color: [u8; 4],
) -> Result<(), Error> {
    for row in rgba
        .chunks_exact_mut(row_bytes)
        .skip(rows.start)
        .take(rows.len())
    {
        let selected = row.get_mut(columns.clone()).ok_or(Error::SizeOverflow)?;

        for pixel in selected.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&color);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapturedImage;
    use claims::{assert_err, assert_ok, assert_some};
    use rstest::rstest;
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

    #[rstest]
    #[case([0, 0, 0, 1])]
    #[case([0, 1, 0, 0])]
    #[case([2, 1, 0, 1])]
    #[case([0, 1, 2, 1])]
    #[case([0, 3, 0, 1])]
    #[case([0, 1, 0, 2])]
    fn rejects_invalid_bounds_without_changing_pixels(#[case] edges: [usize; 4]) {
        let [left, right, top, bottom] = edges;
        let original = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let mut image = image(2, 1, original.clone());

        let error = assert_err!(image.pixelate(left..right, top..bottom, NonZeroU32::MAX));

        assert_matches!(error, Error::InvalidPixelationBounds);
        assert_eq!(image.rgba(), original);
    }
}
