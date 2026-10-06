use std::{num::NonZeroU32, ops::Range};

use crate::{Error, PixelSize};

/// An owned image with tightly packed, top-to-bottom RGBA8 pixels.
///
/// Alpha is straight, not premultiplied.
#[derive(Debug)]
pub struct CapturedImage {
    size: PixelSize,
    rgba: Vec<u8>,
}

impl CapturedImage {
    /// Creates an image from owned RGBA8 pixel storage.
    ///
    /// Takes ownership without copying or allocating pixel storage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SizeOverflow`] if the required storage length
    /// cannot be represented, or [`Error::InvalidPixelLength`] if the
    /// supplied storage length does not match.
    pub fn new(size: PixelSize, rgba: Vec<u8>) -> Result<Self, Error> {
        let expected = size.rgba_len()?;
        let actual = rgba.len();

        if actual != expected {
            return Err(Error::InvalidPixelLength { expected, actual });
        }
        Ok(Self { size, rgba })
    }

    /// Returns the image dimensions in pixels.
    #[must_use]
    pub const fn size(&self) -> PixelSize {
        self.size
    }

    /// Returns tightly packed RGBA8 pixels in row order.
    #[must_use]
    pub const fn rgba(&self) -> &[u8] {
        self.rgba.as_slice()
    }

    /// Copies a nonempty region into a new tightly packed RGBA8 image.
    ///
    /// Coordinates are pixel ranges in this image's current buffer orientation.
    /// Range ends are exclusive. The source pixels and straight alpha are preserved.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCropBounds`] for empty, reversed, or out-of-bounds
    /// ranges, [`Error::SizeOverflow`] if dimensions or offsets cannot be
    /// represented, or [`Error::Allocation`] if allocating the result fails.
    pub fn crop(&self, x: Range<usize>, y: Range<usize>) -> Result<Self, Error> {
        let source_width =
            usize::try_from(self.size.width().get()).map_err(|_| Error::SizeOverflow)?;
        let source_height =
            usize::try_from(self.size.height().get()).map_err(|_| Error::SizeOverflow)?;

        if x.is_empty() || y.is_empty() || x.end > source_width || y.end > source_height {
            return Err(Error::InvalidCropBounds);
        }

        let width = u32::try_from(x.len())
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(Error::SizeOverflow)?;
        let height = u32::try_from(y.len())
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(Error::SizeOverflow)?;
        let size = PixelSize::new(width, height);

        let row_bytes = source_width.checked_mul(4).ok_or(Error::SizeOverflow)?;
        let start = x.start.checked_mul(4).ok_or(Error::SizeOverflow)?;
        let end = x.end.checked_mul(4).ok_or(Error::SizeOverflow)?;

        let mut rgba = Vec::new();
        rgba.try_reserve_exact(size.rgba_len()?)?;

        for row in self
            .rgba
            .chunks_exact(row_bytes)
            .skip(y.start)
            .take(y.len())
        {
            let selected = row.get(start..end).ok_or(Error::InvalidCropBounds)?;
            rgba.extend_from_slice(selected);
        }

        Self::new(size, rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some};
    use rstest::rstest;
    use std::assert_matches;

    fn pixels(width: u32, height: u32) -> PixelSize {
        PixelSize::new(
            assert_some!(NonZeroU32::new(width)),
            assert_some!(NonZeroU32::new(height)),
        )
    }

    fn fixture() -> CapturedImage {
        let rgba = (0..12).flat_map(|value| [value, 20, 30, value]).collect();
        assert_ok!(CapturedImage::new(pixels(4, 3), rgba))
    }

    #[test]
    fn copies_selected_rows_and_columns_without_changing_the_source() {
        let image = fixture();
        let original = image.rgba().to_vec();
        let cropped = assert_ok!(image.crop(1..3, 1..3));
        let expected = [5, 6, 9, 10]
            .into_iter()
            .flat_map(|value| [value, 20, 30, value])
            .collect::<Vec<_>>();

        assert_eq!(cropped.size(), pixels(2, 2));
        assert_eq!(cropped.rgba(), expected);
        assert_eq!(image.rgba(), original);
    }

    #[test]
    fn accepts_the_entire_image() {
        let image = fixture();
        let cropped = assert_ok!(image.crop(3..4, 2..3));

        assert_eq!(cropped.size(), pixels(1, 1));
        assert_eq!(cropped.rgba(), [11, 20, 30, 11]);
    }

    #[rstest]
    #[case([0, 0, 0, 1])]
    #[case([0, 1, 0, 0])]
    #[case([2, 1, 0, 1])]
    #[case([0, 1, 2, 1])]
    #[case([0, 5, 0, 1])]
    #[case([0, 1, 0, 4])]
    fn rejects_invalid_bounds(#[case] edges: [usize; 4]) {
        let [left, right, top, bottom] = edges;
        let error = assert_err!(fixture().crop(left..right, top..bottom));

        assert_matches!(error, Error::InvalidCropBounds);
    }
}
