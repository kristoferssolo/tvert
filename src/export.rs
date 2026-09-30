use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use png::{BitDepth, ColorType, Encoder};
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
    let expected_len = image
        .size
        .rgba_len()
        .wrap_err("image dimensions exceed the supported size")?;

    if image.rgba.len() != expected_len {
        bail!(
            "invalid RBGA image length: expected {expected_len}, got {}",
            image.rgba.len()
        );
    }

    let file =
        File::create_new(path).wrap_err_with(|| format!("failed to create {}", path.display()))?;
    let mut output = BufWriter::new(file);

    let mut encoder = Encoder::new(
        &mut output,
        image.size.width().get(),
        image.size.height().get(),
    );
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

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
