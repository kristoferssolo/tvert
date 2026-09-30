use crate::{Error, PixelSize};

/// An owned image with tightly packed, top-to-bottom RGBA8 pixels.
///
/// Alpha is straight, not premultiplied.
#[derive(Debug)]
pub struct CapturedImage {
    pub size: PixelSize,
    /// Consecutive red, green, blue, and alpha bytes, without row padding
    pub rgba: Vec<u8>,
}

impl CapturedImage {
    /// Checks that pixel storage matches the image dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SizeOverflow`] if the required storage length
    /// cannot be represented, or [`Error::InvalidPixelLength`] if the
    /// pixel length does not match.
    pub fn validate(&self) -> Result<(), Error> {
        let expected = self.size.rgba_len()?;
        let actual = self.rgba.len();

        if actual != expected {
            return Err(Error::InvalidPixelLength { expected, actual });
        }

        Ok(())
    }
}
