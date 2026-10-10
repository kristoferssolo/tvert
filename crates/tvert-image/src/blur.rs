use crate::{
    Error, PixelSize,
    color::{average_color, weighted_channels},
    region::RgbaRegion,
};
use std::{num::NonZeroU32, ops::Range};

pub fn blur(
    rgba: &mut [u8],
    size: PixelSize,
    x: Range<usize>,
    y: Range<usize>,
    radius: NonZeroU32,
) -> Result<(), Error> {
    let region = RgbaRegion::new(size, x, y)?.ok_or(Error::InvalidBlurBounds)?;
    let (width, height) = region.size().try_into()?;

    let radius = usize::try_from(radius.get()).map_err(|_| Error::SizeOverflow)?;
    let horizontal = horizontal_sums(rgba, &region, radius)?;

    let mut output = Vec::new();
    let len = region.size().rgba_len()?;

    output.try_reserve_exact(len)?;
    output.resize(len, 0);

    blur_columns(&horizontal, &mut output, width, height, radius)?;

    for (row, blurred) in region
        .rows_mut(rgba)
        .zip(output.chunks_exact(region.size().row_byes()?))
    {
        row?.copy_from_slice(blurred);
    }
    Ok(())
}

fn horizontal_sums(
    rgba: &[u8],
    region: &RgbaRegion,
    radius: usize,
) -> Result<Vec<[u64; 4]>, Error> {
    let (width, height): (usize, usize) = region.size().try_into()?;
    let count = width.checked_mul(height).ok_or(Error::SizeOverflow)?;

    let mut horizontal = Vec::new();
    horizontal.try_reserve_exact(count)?;

    for row in region.rows(rgba) {
        let pixels = row?.as_chunks::<4>().0;
        blur_line(
            width,
            radius,
            |column| weighted_channels(*pixels.get(column).ok_or(Error::SizeOverflow)?),
            |_, sums, _| {
                let mut channels = [0; 4];
                for (channel, sum) in channels.iter_mut().zip(sums) {
                    *channel = u64::try_from(sum).map_err(|_| Error::SizeOverflow)?;
                }
                horizontal.push(channels);
                Ok(())
            },
        )?;
    }

    Ok(horizontal)
}

fn blur_columns(
    horizontal: &[[u64; 4]],
    output: &mut [u8],
    width: usize,
    height: usize,
    radius: usize,
) -> Result<(), Error> {
    for column in 0..width {
        let columns =
            u128::try_from(window(column, width, radius).len()).map_err(|_| Error::SizeOverflow)?;
        blur_line(
            height,
            radius,
            |row| {
                let index = offset(row, column, width)?;
                horizontal.get(index).copied().ok_or(Error::SizeOverflow)
            },
            |row, sums, rows| {
                let count = columns
                    .checked_mul(u128::try_from(rows).map_err(|_| Error::SizeOverflow)?)
                    .ok_or(Error::SizeOverflow)?;
                let index = offset(row, column, width)?;
                let pixel = output
                    .as_chunks_mut::<4>()
                    .0
                    .get_mut(index)
                    .ok_or(Error::SizeOverflow)?;
                *pixel = average_color(sums, count)?;
                Ok(())
            },
        )?;
    }
    Ok(())
}

fn offset(row: usize, column: usize, width: usize) -> Result<usize, Error> {
    row.checked_mul(width)
        .and_then(|value| value.checked_add(column))
        .ok_or(Error::SizeOverflow)
}

fn window(center: usize, length: usize, radius: usize) -> Range<usize> {
    center.saturating_sub(radius)..center.saturating_add(radius).saturating_add(1).min(length)
}

fn blur_line(
    length: usize,
    radius: usize,
    mut source: impl FnMut(usize) -> Result<[u64; 4], Error>,
    mut output: impl FnMut(usize, [u128; 4], usize) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut sums = [0u128; 4];
    let mut left = 0;
    let mut right = 0;
    for center in 0..length {
        let bounds = window(center, length, radius);
        while right < bounds.end {
            for (sum, channel) in sums.iter_mut().zip(source(right)?) {
                *sum = sum
                    .checked_add(u128::from(channel))
                    .ok_or(Error::SizeOverflow)?;
            }
            right = right.checked_add(1).ok_or(Error::SizeOverflow)?;
        }
        while left < bounds.start {
            for (sum, channel) in sums.iter_mut().zip(source(left)?) {
                *sum = sum
                    .checked_sub(u128::from(channel))
                    .ok_or(Error::SizeOverflow)?;
            }
            left = left.checked_add(1).ok_or(Error::SizeOverflow)?;
        }
        output(center, sums, bounds.len())?;
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
    fn blurs_in_two_dimensions_without_changing_surrounding_pixels() {
        let reds = [
            200, 200, 200, 200, 200, 200, 0, 0, 0, 200, 200, 0, 255, 0, 200, 200, 0, 0, 0, 200,
            200, 200, 200, 200, 200,
        ];
        let rgba = reds.into_iter().flat_map(|red| [red, 0, 0, 255]).collect();
        let mut image = image(5, 5, rgba);
        assert_ok!(image.blur(1..4, 1..4, assert_some!(NonZeroU32::new(1))));
        let expected = [
            200, 200, 200, 200, 200, 200, 63, 42, 63, 200, 200, 42, 28, 42, 200, 200, 63, 42, 63,
            200, 200, 200, 200, 200, 200,
        ]
        .into_iter()
        .flat_map(|red| [red, 0, 0, 255])
        .collect::<Vec<_>>();
        assert_eq!(image.rgba(), expected);
    }

    #[rstest]
    #[case(1, [15, 30, 60, 90, 105])]
    #[case(u32::MAX, [60; 5])]
    fn moves_the_window_and_clips_large_radii(#[case] radius: u32, #[case] expected: [u8; 5]) {
        let rgba = [0, 30, 60, 90, 120]
            .into_iter()
            .flat_map(|red| [red, 0, 0, 255])
            .collect();
        let mut image = image(5, 1, rgba);
        assert_ok!(image.blur(0..5, 0..1, assert_some!(NonZeroU32::new(radius))));
        let expected = expected
            .into_iter()
            .flat_map(|red| [red, 0, 0, 255])
            .collect::<Vec<_>>();
        assert_eq!(image.rgba(), expected);
    }

    #[rstest]
    #[case(255, [255, 0, 0, 127])]
    #[case(0, [0; 4])]
    fn transparent_pixels_do_not_tint_the_blur(#[case] alpha: u8, #[case] expected: [u8; 4]) {
        let mut image = image(2, 1, vec![255, 0, 0, alpha, 0, 0, 255, 0]);
        assert_ok!(image.blur(0..2, 0..1, NonZeroU32::MAX));
        assert_eq!(image.rgba(), expected.repeat(2));
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
        let error = assert_err!(image.blur(left..right, top..bottom, NonZeroU32::MAX));
        assert_matches!(error, Error::InvalidBlurBounds);
        assert_eq!(image.rgba(), original);
    }
}
