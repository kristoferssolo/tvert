use crate::{CapturedImage, Error};

/// Pixel rotations and reflections, independent of any display protocol.
///
/// Rotations are counterclockwise as viewed in the image.
/// Flipped variants mirror left-to-right before rotating.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageTransform {
    #[default]
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

impl CapturedImage {
    /// Consumes the image and transforms its pixels.
    ///
    /// Quarter turns exchange width and height. RGBA values, including straight
    /// alpha, are copied unchanged. `Normal` returns the original storage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SizeOverflow`] if pixel addressing cannot be represented,
    /// or [`Error::Allocation`] if allocating the transformed storage fails.
    pub fn transformed(self, transform: ImageTransform) -> Result<Self, Error> {
        if transform == ImageTransform::Normal {
            return Ok(self);
        }

        let source_size = self.size();
        let (width, height): (usize, usize) = source_size.try_into()?;
        let last_x = width.checked_sub(1).ok_or(Error::SizeOverflow)?;
        let last_y = height.checked_sub(1).ok_or(Error::SizeOverflow)?;
        let row_bytes = source_size.row_bytes()?;

        let destination_size = source_size.transformed(transform);
        let (destination_width, _) = destination_size.try_into()?;
        let byte_len = destination_size.rgba_len()?;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(byte_len)?;
        rgba.resize(byte_len, 0);

        for (y, row) in self.rgba().chunks_exact(row_bytes).enumerate() {
            let reversed_y = last_y.checked_sub(y).ok_or(Error::SizeOverflow)?;

            for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
                let reversed_x = last_x.checked_sub(x).ok_or(Error::SizeOverflow)?;
                let (destination_x, destination_y) = match transform {
                    ImageTransform::Normal => (x, y),
                    ImageTransform::Rotate90 => (y, reversed_x),
                    ImageTransform::Rotate180 => (reversed_x, reversed_y),
                    ImageTransform::Rotate270 => (reversed_y, x),
                    ImageTransform::Flipped => (reversed_x, y),
                    ImageTransform::Flipped90 => (y, x),
                    ImageTransform::Flipped180 => (x, reversed_y),
                    ImageTransform::Flipped270 => (reversed_y, reversed_x),
                };
                let start = destination_y
                    .checked_mul(destination_width)
                    .and_then(|row| row.checked_add(destination_x))
                    .and_then(|pixel| pixel.checked_mul(4))
                    .ok_or(Error::SizeOverflow)?;
                let end = start.checked_add(4).ok_or(Error::SizeOverflow)?;
                let destination = rgba.get_mut(start..end).ok_or(Error::SizeOverflow)?;
                destination.copy_from_slice(pixel);
            }
        }

        Self::new(destination_size, rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PixelSize;
    use claims::{assert_ok, assert_some};
    use rstest::rstest;
    use std::num::NonZeroU32;

    fn pixels(width: u32, height: u32) -> PixelSize {
        PixelSize::new(
            assert_some!(NonZeroU32::new(width)),
            assert_some!(NonZeroU32::new(height)),
        )
    }

    fn image(width: u32, height: u32, values: &[u8]) -> CapturedImage {
        let rgba = values
            .iter()
            .flat_map(|value| [*value, 20, 30, *value])
            .collect();
        assert_ok!(CapturedImage::new(pixels(width, height), rgba))
    }

    // Original image:
    // 1 2 3
    // 4 5 6
    #[rstest]
    #[case(ImageTransform::Normal, 3, 2, [1, 2, 3, 4, 5, 6])]
    #[case(ImageTransform::Rotate90, 2, 3, [3, 6, 2, 5, 1, 4])]
    #[case(ImageTransform::Rotate180, 3, 2, [6, 5, 4, 3, 2, 1])]
    #[case(ImageTransform::Rotate270, 2, 3, [4, 1, 5, 2, 6, 3])]
    #[case(ImageTransform::Flipped, 3, 2, [3, 2, 1, 6, 5, 4])]
    #[case(ImageTransform::Flipped90, 2, 3, [1, 4, 2, 5, 3, 6])]
    #[case(ImageTransform::Flipped180, 3, 2, [4, 5, 6, 1, 2, 3])]
    #[case(ImageTransform::Flipped270, 2, 3, [6, 3, 5, 2, 4, 1])]
    fn transforms_non_square_images(
        #[case] transform: ImageTransform,
        #[case] width: u32,
        #[case] height: u32,
        #[case] expected: [u8; 6],
    ) {
        let result = assert_ok!(image(3, 2, &[1, 2, 3, 4, 5, 6]).transformed(transform));
        let expected = image(width, height, &expected);

        assert_eq!(result.size(), expected.size());
        assert_eq!(result.rgba(), expected.rgba());
    }

    #[test]
    fn normal_transform_retains_the_allocation() {
        let original = image(3, 2, &[1, 2, 3, 4, 5, 6]);
        let storage = original.rgba().as_ptr();
        let result = assert_ok!(original.transformed(ImageTransform::Normal));

        assert_eq!(result.rgba().as_ptr(), storage);
    }

    #[rstest]
    #[case(1, 3, 3, 1, [1, 2, 3])]
    #[case(3, 1, 1, 3, [3, 2, 1])]
    fn rotates_single_pixel_rows_and_columns(
        #[case] width: u32,
        #[case] height: u32,
        #[case] expected_width: u32,
        #[case] expected_height: u32,
        #[case] expected: [u8; 3],
    ) {
        let result =
            assert_ok!(image(width, height, &[1, 2, 3]).transformed(ImageTransform::Rotate90));
        let expected = image(expected_width, expected_height, &expected);

        assert_eq!(result.size(), expected.size());
        assert_eq!(result.rgba(), expected.rgba());
    }
}
