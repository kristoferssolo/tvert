use crate::{Error, PixelSize, blur::blur, pixelate::pixelate, region::RgbaRegion};
use std::{num::NonZeroU32, ops::Range};

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
        size.validate_rbga(&rgba)?;
        Ok(Self { size, rgba })
    }

    /// Returns the image dimensions in pixels.
    #[must_use]
    pub const fn size(&self) -> PixelSize {
        self.size
    }

    /// Returns tightly packed RGBA8 pixels in row order
    #[must_use]
    pub const fn rgba(&self) -> &[u8] {
        self.rgba.as_slice()
    }

    /// Returns the owned RGBA8 pixels without copying them
    #[must_use]
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
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
        let region = RgbaRegion::new(self.size, x, y)?.ok_or(Error::InvalidCropBounds)?;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(region.size().rgba_len()?)?;
        for row in region.rows(&self.rgba) {
            rgba.extend_from_slice(row?);
        }

        Self::new(region.size(), rgba)
    }

    /// Pixelates a region in place using square blocks of captured-image pixels.
    ///
    /// Range ends are exclusive. Blocks start at the region's top-left corner
    /// and are clipped at its edges. Pixels outside the region are unchanged.
    /// Colors are averaged with alpha weighting and stored with straight alpha.
    /// A block size of one leaves visible pixels unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPixelationBounds`] for empty, reversed, or
    /// out-of-bounds ranges, or [`Error::SizeOverflow`] if dimensions, offsets,
    /// or channel totals cannot be represented.
    pub fn pixelate(
        &mut self,
        x: Range<usize>,
        y: Range<usize>,
        block_size: NonZeroU32,
    ) -> Result<(), Error> {
        pixelate(&mut self.rgba, self.size, x, y, block_size)
    }

    /// Applies a Gaussian blur to a region in place.
    ///
    /// Sigma controls the blur's spread in captured-image pixels. Range ends are
    /// exclusive. Pixels outside the region are neither sampled nor changed.
    /// Channels are filtered in their stored color space and retain straight alpha.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBlurBounds`] for empty, reversed, or out-of-bounds
    /// ranges, [`Error::InvalidBlurSigma`] if sigma is not positive and normal,
    /// [`Error::SizeOverflow`] if the working image cannot be represented, or
    /// [`Error::Allocation`] if allocating the selected-region buffer fails.
    /// The image remains unchanged on error.
    pub fn blur(&mut self, x: Range<usize>, y: Range<usize>, sigma: f32) -> Result<(), Error> {
        blur(&mut self.rgba, self.size, x, y, sigma)
    }
}

impl TryFrom<(PixelSize, &[u8])> for CapturedImage {
    type Error = Error;

    fn try_from((size, rgba): (PixelSize, &[u8])) -> Result<Self, Self::Error> {
        size.validate_rbga(rgba)?;
        let mut copy = Vec::new();
        copy.try_reserve_exact(rgba.len())?;
        copy.extend_from_slice(rgba);
        Ok(Self { size, rgba: copy })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use rstest::rstest;
    use std::assert_matches;

    fn pixels(width: u32, height: u32) -> PixelSize {
        assert_ok!(PixelSize::try_from((width, height)))
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
