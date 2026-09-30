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
}
