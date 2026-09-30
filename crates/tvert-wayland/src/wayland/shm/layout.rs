use crate::{Error, wayland::screencopy::ShmRequirements};
use std::num::NonZeroU32;
use wayland_client::{WEnum, protocol::wl_shm::Format};

/// Validated row stride for a four-byte-per-pixel SHM image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stride(NonZeroU32);

impl Stride {
    /// Validates an advertised stride against the image width.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidShmLayout`] if the minimum row size
    /// overflows, or the stride is too small or not four-byte aligned.
    pub fn new(bytes: u32, width: NonZeroU32) -> Result<Self, Error> {
        let minimum = Self::packed(width)?;

        if bytes < minimum.bytes() || !bytes.is_multiple_of(4) {
            return Err(Error::InvalidShmLayout);
        }

        let bytes = NonZeroU32::new(bytes).ok_or(Error::InvalidShmLayout)?;
        Ok(Self(bytes))
    }

    /// Creates a tightly packed stride for the image width.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidShmLayout`] if the row size overflows.
    pub fn packed(width: NonZeroU32) -> Result<Self, Error> {
        let bytes = width
            .get()
            .checked_mul(4)
            .and_then(NonZeroU32::new)
            .ok_or(Error::InvalidShmLayout)?;
        Ok(Self(bytes))
    }

    pub const fn bytes(self) -> u32 {
        self.0.get()
    }

    /// Calculates the backing buffer length in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidShmLayout`] if the length exceeds
    /// the positive `i32` range required by `wl_shm_pool`.
    pub fn buffer_len(self, height: NonZeroU32) -> Result<i32, Error> {
        self.bytes()
            .checked_mul(height.get())
            .and_then(|bytes| i32::try_from(bytes).ok())
            .ok_or(Error::InvalidShmLayout)
    }

    /// Calculates a row's byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidShmLayout`] if the offset overflows.
    pub fn row_offset(self, row: u32) -> Result<u64, Error> {
        u64::from(self.bytes())
            .checked_mul(u64::from(row))
            .ok_or(Error::InvalidShmLayout)
    }
}

/// Validated layout for a supported four-byte-per-pixel SHM image.
///
/// Fields are private so dimensions and stride cannot be changed
/// independently after validation.
#[derive(Debug, Clone, Copy)]
pub struct ShmLayout {
    format: Format,
    width: NonZeroU32,
    height: NonZeroU32,
    stride: Stride,
    pool_size: i32,
    file_len: u64,
    row_bytes: usize,
    rgba_len: usize,
}

impl ShmLayout {
    /// Validates raw SHM requirements and calculates storage sizes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedShmFormat`] for unsupported formats.
    ///
    /// Returns [`Error::InvalidShmLayout`] for zero dimensions, invalid
    /// stride, arithmetic overflow, or sizes that cannot be represented
    /// by Wayland or this process.
    pub fn new(format: WEnum<Format>, width: u32, height: u32, stride: u32) -> Result<Self, Error> {
        let format = match format {
            WEnum::Value(
                format
                @ (Format::Argb8888 | Format::Xrgb8888 | Format::Abgr8888 | Format::Xbgr8888),
            ) => format,
            other => return Err(Error::UnsupportedShmFormat(other)),
        };

        let width = NonZeroU32::new(width).ok_or(Error::InvalidShmLayout)?;
        let height = NonZeroU32::new(height).ok_or(Error::InvalidShmLayout)?;
        let stride = Stride::new(stride, width)?;

        let pool_size = stride.buffer_len(height)?;
        let file_len = u64::try_from(pool_size).map_err(|_| Error::InvalidShmLayout)?;

        let packed_stride = Stride::packed(width)?;
        let row_bytes =
            usize::try_from(packed_stride.bytes()).map_err(|_| Error::InvalidShmLayout)?;
        let rgba_len = usize::try_from(packed_stride.buffer_len(height)?)
            .map_err(|_| Error::InvalidShmLayout)?;

        Ok(Self {
            format,
            width,
            height,
            stride,
            pool_size,
            file_len,
            row_bytes,
            rgba_len,
        })
    }

    pub const fn format(self) -> Format {
        self.format
    }

    pub const fn width(self) -> NonZeroU32 {
        self.width
    }

    pub const fn height(self) -> NonZeroU32 {
        self.height
    }

    pub const fn stride(self) -> Stride {
        self.stride
    }

    /// Backing pool size in the representation required by Wayland
    pub const fn pool_size(self) -> i32 {
        self.pool_size
    }

    /// Backing file length, including row padding
    pub const fn file_len(self) -> u64 {
        self.file_len
    }

    /// Bytes of pixel content in one row, excluding padding
    pub const fn row_bytes(self) -> usize {
        self.row_bytes
    }

    /// Length of a tightly packed RGBA8 image.
    pub const fn rgba_len(self) -> usize {
        self.rgba_len
    }

    /// Calculates the backing-file offset of a row.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidShmLayout`] if the row is outside the image
    /// or its offset cannot be represented.
    pub fn row_offset(self, row: u32) -> Result<u64, Error> {
        if row >= self.height.get() {
            return Err(Error::InvalidShmLayout);
        }

        self.stride.row_offset(row)
    }
}

impl TryFrom<ShmRequirements> for ShmLayout {
    type Error = Error;

    fn try_from(requirements: ShmRequirements) -> Result<Self, Self::Error> {
        Self::new(
            requirements.format,
            requirements.width,
            requirements.height,
            requirements.stride,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_matches, assert_ok};
    use rstest::rstest;

    #[test]
    fn distinguishes_backing_storage_from_packed_pixels() {
        let layout = assert_ok!(ShmLayout::new(WEnum::Value(Format::Xrgb8888), 1, 2, 8));

        assert_eq!(layout.pool_size(), 16);
        assert_eq!(layout.file_len(), 16);
        assert_eq!(layout.row_bytes(), 4);
        assert_eq!(layout.rgba_len(), 8);
        assert_eq!(assert_ok!(layout.row_offset(1)), 8);
        assert_matches!(assert_err!(layout.row_offset(2)), Error::InvalidShmLayout);
    }

    #[rstest]
    #[case(1, 1, 6)]
    #[case(536_870_912, 1, 2_147_483_648)]
    #[case(1, 536_870_912, 4)]
    fn rejects_invalid_stride_or_oversized_pool(
        #[case] width: u32,
        #[case] height: u32,
        #[case] stride: u32,
    ) {
        assert_matches!(
            assert_err!(ShmLayout::new(
                WEnum::Value(Format::Xrgb8888),
                width,
                height,
                stride
            )),
            Error::InvalidShmLayout
        );
    }
}
