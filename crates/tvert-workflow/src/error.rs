use std::{io, path::PathBuf};

/// An error while capturing or exporting an image
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to capture output")]
    Capture(#[source] tvert_wayland::Error),

    #[error("the compositor reported no outputs")]
    NoOutputs,

    #[error("no output named {name:?}; available outputs: {}", .available.join(", "))]
    UnknownOutput {
        name: String,
        available: Vec<String>,
    },

    /// A command that needs one output received `all` with several outputs
    #[error(
        "several outputs are available; choose one of {} with --output NAME or capture.output",
        .available.join(", ")
    )]
    AmbiguousOutput { available: Vec<String> },

    #[error(
        "the desktop position of output {name:?} is unknown; \
         combined capture requires zxdg_output_manager_v1"
    )]
    UnknownPlacement { name: String },

    /// Outputs were added, removed, or rearranged during capture or selection
    #[error("outputs changed during capture or selection")]
    OutputsChanged,

    #[error("the desktop layout exceeds the supported image size")]
    DesktopTooLarge,

    /// Allocating or painting the combined desktop image failed
    #[error("failed to combine the captured outputs")]
    Compose(#[source] tvert_image::Error),

    /// The selected region missed the combined desktop image, or copying it failed
    #[error("failed to crop the selected desktop region")]
    Crop(#[source] tvert_image::Error),

    #[error("failed to create PNG destination {path:?}")]
    CreateDestination {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to create save directory {path:?}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The generated filename and every numbered alternative are taken
    #[error("no available PNG filename for {path:?}")]
    FilenamesExhausted { path: PathBuf },

    /// Expanding the save path pattern did not produce a PNG destination
    #[error("invalid generated save path")]
    Pattern(#[from] PatternError),

    /// Encoding or writing PNG data failed
    #[error("failed to encode PNG for {path:?}")]
    EncodePng {
        path: PathBuf,
        #[source]
        source: tvert_image::Error,
    },

    #[error("failed to flush PNG destination {path:?}")]
    FlushDestination {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to encode clipboard PNG")]
    EncodeClipboardPng(#[source] tvert_image::Error),

    /// Clipboard setup or a paste transfer failed
    #[error("failed to access the Wayland clipboard")]
    Clipboard(#[from] wl_clipboard_rs::copy::Error),
}

/// An empty output name
#[derive(Debug, thiserror::Error)]
#[error(r#"expected an output name, such as DP-1, or "all""#)]
pub struct EmptyOutputName;

/// A save path pattern that cannot produce a PNG destination
#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("expected a nonempty path pattern")]
    Empty,

    #[error("path patterns cannot contain NUL characters")]
    Nul,

    #[error("a leading ~/ requires HOME to be an absolute path")]
    MissingHome,

    /// The pattern contains an invalid strftime directive
    #[error(transparent)]
    Format(#[from] jiff::Error),

    #[error("expanded path {path:?} does not end with a filename")]
    MissingFilename { path: PathBuf },

    #[error("expanded path {path:?} is not a .png file")]
    NotPng { path: PathBuf },
}

impl From<tvert_wayland::Error> for Error {
    fn from(error: tvert_wayland::Error) -> Self {
        match error {
            tvert_wayland::Error::NoOutputs => Self::NoOutputs,
            tvert_wayland::Error::OutputsChanged => Self::OutputsChanged,
            tvert_wayland::Error::UnknownPlacement { name } => Self::UnknownPlacement { name },
            other => Self::Capture(other),
        }
    }
}
