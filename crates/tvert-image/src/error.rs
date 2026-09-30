/// An error calculating or processing image data.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Pixel dimensions exceed the addressable RGBA image size
    #[error("image dimensions exceed the supported RGBA size")]
    SizeOverflow,
}
