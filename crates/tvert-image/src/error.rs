use std::collections::TryReserveError;

/// An error calculating, processing, or encoding image data
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Pixel dimensions exceed the addressable RGBA image size
    #[error("image dimensions exceed the supported RGBA size")]
    SizeOverflow,

    /// Pixel storage does not match the image dimensions
    #[error("invalid RGBA image length: expected {expected}, got {actual}")]
    InvalidPixelLength { expected: usize, actual: usize },

    /// Encoding PNG data or writing it to the destination failed
    #[error("failed to encode PNG")]
    PngEncoding(#[from] png::EncodingError),

    /// The crop is empty, reversed, or extends outside the source image
    #[error("crop bounds must describe a nonempty region inside the image")]
    InvalidCropBounds,

    /// Allocating image storage failed
    #[error("failed to allocate image storage")]
    Allocation(#[from] TryReserveError),

    /// At least one pixel dimension is zero
    #[error("image dimensions must be nonzero")]
    ZeroDimensions,

    /// The pixelation region is empty, reversed, or outside the image
    #[error("pixelation bounds must describe a nonempty region inside the image")]
    InvalidPixelationBounds,
}
