use crate::Error;
use std::num::NonZeroU32;

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
        let width = usize::try_from(self.width.get()).map_err(|_| Error::SizeOverflow)?;
        let height = usize::try_from(self.height.get()).map_err(|_| Error::SizeOverflow)?;

        width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(Error::SizeOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use std::assert_matches;

    #[test]
    fn rejects_unrepresentable_rgba_storage() {
        let size = PixelSize::new(NonZeroU32::MAX, NonZeroU32::MAX);

        assert_matches!(assert_err!(size.rgba_len()), Error::SizeOverflow);
    }
}
