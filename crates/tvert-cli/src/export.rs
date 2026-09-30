use color_eyre::{Result, eyre::Context};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use tvert_image::{CapturedImage, write_png};

/// Saves an RGBA8 image as a PNG without replacing an existing file.
///
/// # Errors
///
/// Returns an error if the image layout is invalid, the destination cannot
/// be created, or encoding or writing fails.
pub fn save_png(path: &Path, image: &CapturedImage) -> Result<()> {
    image.validate().wrap_err("invalid image fro PNG export")?;

    let file =
        File::create_new(path).wrap_err_with(|| format!("failed to create {}", path.display()))?;
    let mut output = BufWriter::new(file);

    write_png(&mut output, image)
        .wrap_err_with(|| format!("failed to encode PNG for {}", path.display()))?;

    output
        .flush()
        .wrap_err_with(|| format!("failed to flush {}", path.display()))?;

    Ok(())
}
