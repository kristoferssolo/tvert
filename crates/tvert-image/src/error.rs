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
}
