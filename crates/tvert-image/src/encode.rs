use crate::{CapturedImage, Error};
use png::{BitDepth, ColorType, Encoder};
use std::io::Write;

/// Writes an RGBA8 image as a complete PNG.
///
/// Validates the image before writing any bytes.
/// Accepts any [`Write`] destination, including a file or byte vector.
///
/// # Errors
///
/// Returns [`Error::SizeOverflow`] or [`Error::InvalidPixelLength`]
/// if the image layout is invalid.
///
/// Returns [`Error::PngEncoding`] if encoding or writing fails.
pub fn write_png(output: impl Write, image: &CapturedImage) -> Result<(), Error> {
    let (width, height) = image.size().into();

    let mut encoder = Encoder::new(output, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    writer.write_image_data(image.rgba())?;
    writer.finish()?;

    Ok(())
}
