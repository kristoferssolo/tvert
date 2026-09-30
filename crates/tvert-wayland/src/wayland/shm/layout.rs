use crate::Error;
use std::num::NonZeroU32;

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
