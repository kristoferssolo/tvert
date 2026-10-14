use std::collections::TryReserveError;

/// An error calculating, processing, or encoding image data
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("image dimensions exceed the supported RGBA size")]
    SizeOverflow,

    #[error("invalid RGBA image length: expected {expected}, got {actual}")]
    InvalidPixelLength { expected: usize, actual: usize },

    /// Encoding PNG data or writing it to the destination failed
    #[error("failed to encode PNG")]
    PngEncoding(#[from] png::EncodingError),

    #[error("crop bounds must describe a nonempty region inside the image")]
    InvalidCropBounds,

    #[error("failed to allocate image storage")]
    Allocation(#[from] TryReserveError),

    #[error("image dimensions must be nonzero")]
    ZeroDimensions,

    #[error("pixelation bounds must describe a nonempty region inside the image")]
    InvalidPixelationBounds,

    #[error("blur bounds must describe a nonempty region inside the image")]
    InvalidBlurBounds,

    #[error("paint bounds must describe a nonempty region inside the image")]
    InvalidPaintBounds,

    #[error("blur sigma must be positive, finite, and not subnormal")]
    InvalidBlurSigma,
}
