use std::{io, path::PathBuf};

/// An error while capturing or saving an image.
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
}
