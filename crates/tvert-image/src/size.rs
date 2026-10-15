use crate::{Error, ImageTransform};
use std::{fmt::Display, num::NonZeroU32};

/// Nonzero dimensions measured in pixels.
///
/// This describes the pixel buffer's dimensions, regardless of its
/// orientation or the surface size used to display it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

impl PixelSize {
    #[must_use]
    pub const fn new(width: NonZeroU32, height: NonZeroU32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(self) -> NonZeroU32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> NonZeroU32 {
        self.height
    }

    /// Calculates the length of tightly packed RGBA8 storage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SizeOverflow`] if the dimensions or byte
    /// length cannot be represented by this process.
    pub fn rgba_len(self) -> Result<usize, Error> {
        let (_, height) = self.try_into()?;
        self.row_bytes()?
            .checked_mul(height)
            .ok_or(Error::SizeOverflow)
    }

    /// Returns the dimensions after applying an image transform.
    ///
    /// Quarter turns exchange width and height. Other transforms preserve them.
    #[must_use]
    pub const fn transformed(self, transform: ImageTransform) -> Self {
        if transform.swaps_axes() {
            Self::new(self.height, self.width)
        } else {
            self
        }
    }

    /// Returns the byte length of one tightly packed RGBA8 row.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SizeOverflow`] if the row length cannot be represented.
    pub fn row_bytes(self) -> Result<usize, Error> {
        let (width, _): (usize, usize) = self.try_into()?;
        width.checked_mul(4).ok_or(Error::SizeOverflow)
    }

    /// Validates the storage length of a tightly packed RGBA8 buffer.
    ///
    /// This checks layout only, regardless of how alpha is represented.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SizeOverflow`] if the expected length cannot be
    /// represented, or [`Error::InvalidPixelLength`] if the lengths differ.
    pub fn validate_rgba(self, rgba: &[u8]) -> Result<(), Error> {
        let expected = self.rgba_len()?;
        let actual = rgba.len();
        if actual != expected {
            return Err(Error::InvalidPixelLength { expected, actual });
        }
        Ok(())
    }
}

impl TryFrom<(u32, u32)> for PixelSize {
    type Error = Error;
    fn try_from((width, height): (u32, u32)) -> Result<Self, Self::Error> {
        let width = NonZeroU32::new(width).ok_or(Error::ZeroDimensions)?;
        let height = NonZeroU32::new(height).ok_or(Error::ZeroDimensions)?;

        Ok(Self::new(width, height))
    }
}

impl From<PixelSize> for (u32, u32) {
    fn from(size: PixelSize) -> Self {
        (size.width.get(), size.height.get())
    }
}

impl TryFrom<PixelSize> for (i32, i32) {
    type Error = Error;

    fn try_from(size: PixelSize) -> Result<Self, Self::Error> {
        Ok((
            size.width
                .get()
                .try_into()
                .map_err(|_| Error::SizeOverflow)?,
            size.height
                .get()
                .try_into()
                .map_err(|_| Error::SizeOverflow)?,
        ))
    }
}

impl TryFrom<PixelSize> for (usize, usize) {
    type Error = Error;

    fn try_from(size: PixelSize) -> Result<Self, Self::Error> {
        Ok((
            size.width
                .get()
                .try_into()
                .map_err(|_| Error::SizeOverflow)?,
            size.height
                .get()
                .try_into()
                .map_err(|_| Error::SizeOverflow)?,
        ))
    }
}

impl Display for PixelSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use rstest::rstest;
    use std::assert_matches;

    #[rstest]
    #[case(0, 1)]
    #[case(1, 0)]
    fn rejects_zero_dimensions(#[case] width: u32, #[case] height: u32) {
        assert_matches!(
            assert_err!(PixelSize::try_from((width, height))),
            Error::ZeroDimensions
        );
    }

    #[test]
    fn rejects_unrepresentable_rgba_storage() {
        let size = PixelSize::new(NonZeroU32::MAX, NonZeroU32::MAX);

        assert_matches!(assert_err!(size.rgba_len()), Error::SizeOverflow);
    }
}
