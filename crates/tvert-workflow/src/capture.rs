use crate::{Error, destination::Destination, export::save_png};
use std::path::PathBuf;
use tvert_wayland::{OutputId, Wayland};

/// Captures an output and saves it as a PNG.
///
/// Obtain `output_id` from the supplied connection's output snapshots.
/// Captures before creating the destination and never replaces an
/// existing file. Returns the path of the created file.
///
/// # Errors
///
/// Returns [`Error::Capture`] if capture fails, or any error from
/// [`save_png`] if saving fails.
pub fn capture_output_to_png(
    wayland: &mut Wayland,
    output_id: OutputId,
    destination: &Destination,
) -> Result<PathBuf, Error> {
    let image = wayland.capture_output(output_id)?;
    save_png(destination, &image)
}
