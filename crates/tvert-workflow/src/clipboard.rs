use crate::Error;
use tvert_image::{CapturedImage, write_png};
use wl_clipboard_rs::copy::{MimeType, Options, PreparedCopy, Source};

/// Owns a PNG clipboard offer and its Wayland connection.
///
/// Keep this value alive after the editor closes and call [`Self::serve`]
/// to supply the image to applications that paste it.
pub struct ClipboardCopy {
    prepared: PreparedCopy,
}

impl ClipboardCopy {
    /// Serves paste requests until another application replaces the clipboard.
    ///
    /// This blocks and retains clipboard ownership without an editor window.
    /// Dropping the offer or exiting the process ends ownership.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Clipboard`] if communicating with the compositor or
    /// transferring the PNG fails.
    pub fn serve(self) -> Result<(), Error> {
        self.prepared.serve().map_err(Error::Clipboard)
    }
}

/// Encodes an image and prepares an `image/png` offer for the regular clipboard.
///
/// Requires `ext-data-control-v1` or `wlr-data-control-unstable-v1`.
/// Call [`ClipboardCopy::serve`] after closing the editor to publish the offer
/// and supply its PNG bytes. Preparation does not start a background thread.
///
/// # Errors
///
/// Returns [`Error::EncodeClipboardPng`] if encoding fails, or
/// [`Error::Clipboard`] if establishing the clipboard offer fails.
pub fn prepare_png_copy(image: &CapturedImage) -> Result<ClipboardCopy, Error> {
    let mut png = Vec::new();
    write_png(&mut png, image).map_err(Error::EncodeClipboardPng)?;

    let mut options = Options::new();
    options.foreground(true);
    let prepared = options.prepare_copy(
        Source::Bytes(png.into_boxed_slice()),
        MimeType::Specific("image/png".into()),
    )?;

    Ok(ClipboardCopy { prepared })
}
