use crate::Error;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use tvert_image::{CapturedImage, write_png};

/// Saves an image as a PNG without replacing an existing file.
///
/// # Errors
///
/// Returns [`Error::CreateDestination`] if creating the file fails,
/// [`Error::EncodePng`] if encoding or writing fails, or
/// [`Error::FlushDestination`] if flushing buffered output fails.
pub fn save_png(path: &Path, image: &CapturedImage) -> Result<(), Error> {
    let file = File::create_new(path).map_err(|source| Error::CreateDestination {
        path: path.to_path_buf(),
        source,
    })?;
    let mut output = BufWriter::new(file);

    write_png(&mut output, image).map_err(|source| Error::EncodePng {
        path: path.to_path_buf(),
        source,
    })?;

    output.flush().map_err(|source| Error::FlushDestination {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(())
}
