use crate::{Error, destination::Destination};
use std::{
    io::{BufWriter, Write},
    path::PathBuf,
};
use tvert_image::{CapturedImage, write_png};

/// Saves an image as a PNG without replacing an existing file.
///
/// Returns the path of the created file.
///
/// # Errors
///
/// Returns [`Error::Pattern`] if the expanded path pattern is not a PNG file,
/// [`Error::CreateDirectory`] if creating its parent directories fails,
/// [`Error::FilenamesExhausted`] if every generated filename is taken,
/// [`Error::EncodePng`] if encoding or writing fails, or
/// [`Error::FlushDestination`] if flushing buffered output fails.
pub fn save_png(destination: &Destination, image: &CapturedImage) -> Result<PathBuf, Error> {
    let (path, file) = destination.create()?;
    let mut output = BufWriter::new(file);

    write_png(&mut output, image).map_err(|source| Error::EncodePng {
        path: path.clone(),
        source,
    })?;

    output.flush().map_err(|source| Error::FlushDestination {
        path: path.clone(),
        source,
    })?;

    Ok(path)
}
