use crate::{Error, export::save_png};
use std::path::Path;
use tvert_wayland::{OutputId, Wayland};

/// Captures an output and saves it as a PNG.
///
/// Obtain `output_id` from the supplied connection's output snapshots.
/// Captures before creating the destination and never replaces an
/// existing file.
///
/// # Errors
///
/// Returns [`Error::Capture`] if capture fails,
/// [`Error::CreateDestination`] if creating the destination fails,
/// [`Error::EncodePng`] if encoding or writing fails, or
/// [`Error::FlushDestination`] if flushing buffered output fails.
pub fn capture_output_to_png(
    wayland: &mut Wayland,
    output_id: OutputId,
    path: &Path,
) -> Result<(), Error> {
    let image = wayland.capture_output(output_id)?;
    save_png(path, &image)
}
