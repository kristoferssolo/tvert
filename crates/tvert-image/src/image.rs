use crate::PixelSize;

/// An owned image with tightly packed, top-to-bottom RGBA8 pixels.
///
/// Alpha is straight, not premultiplied.
#[derive(Debug)]
pub struct CapturedImage {
    pub size: PixelSize,
    /// Consecutive red, green, blue, and alpha bytes, without row padding
    pub rgba: Vec<u8>,
}
