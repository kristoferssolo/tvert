use std::{io, path::PathBuf};

/// An error while capturing or exporting an image
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Capturing the selected output failed
    #[error("failed to capture output")]
    Capture(#[from] tvert_wayland::Error),

    /// Creating the destination file failed
    #[error("failed to create PNG destination {path:?}")]
    CreateDestination {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Creating the default save directory failed
    #[error("failed to create save directory {path:?}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Every generated filename in the save directory is already taken
    #[error("no available PNG filename in {directory:?}")]
    FilenamesExhausted { directory: PathBuf },

    /// Encoding or writing PNG data failed
    #[error("failed to encode PNG for {path:?}")]
    EncodePng {
        path: PathBuf,
        #[source]
        source: tvert_image::Error,
    },

    /// Flushing buffered output failed
    #[error("failed to flush PNG destination {path:?}")]
    FlushDestination {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Encoding the clipboard image failed
    #[error("failed to encode clipboard PNG")]
    EncodeClipboardPng(#[source] tvert_image::Error),

    /// Clipboard setup or a paste transfer failed
    #[error("failed to access the Wayland clipboard")]
    Clipboard(#[from] wl_clipboard_rs::copy::Error),
}
