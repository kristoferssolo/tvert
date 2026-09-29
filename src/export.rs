use color_eyre::{
    Result,
    eyre::{Context, Ok, bail, eyre},
};
use png::Encoder;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use tvert_wayland::CapturedImage;

/// Saves an RGBA8 image as a PNG without replacing an existing file.
///
/// # Errors
///
/// Returns an error if the image layout is invalid, the destination cannot
/// be created, or encoding or writing fails.
pub fn save_png(path: &Path, image: &CapturedImage) -> Result<()> {
    let width = usize::try_from(image.width.get())?;
    let height = usize::try_from(image.height.get())?;
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| eyre!("image dimensions exceed the supported size"))?;

    if image.rgba.len() != expected_len {
        bail!(
            "invalid RBGA image length: expected {expected_len}, got {}",
            image.rgba.len()
        );
    }

    let file =
        File::create_new(path).wrap_err_with(|| format!("failed to create {}", path.display()))?;
    let mut output = BufWriter::new(file);

    let mut encoder = Encoder::new(&mut output, image.width.get(), image.height.get());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .wrap_err("failed to write PNG header")?;

    writer
        .write_image_data(&image.rgba)
        .wrap_err("failed to encode PNG pixels")?;

    writer.finish().wrap_err("failed to finish PNG")?;

    output
        .flush()
        .wrap_err_with(|| format!("failed to flush {}", path.display()))?;

    Ok(())
}
