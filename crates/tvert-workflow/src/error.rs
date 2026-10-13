use std::{io, path::PathBuf};

/// An error while capturing or exporting an image
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Capturing the selected output failed
    #[error("failed to capture output")]
    Capture(#[from] tvert_wayland::Error),

    /// The compositor reported no outputs to capture
    #[error("the compositor reported no outputs")]
    NoOutputs,

    /// No output has the requested name
    #[error("no output named {name:?}; available outputs: {}", .available.join(", "))]
    UnknownOutput {
        name: String,
        available: Vec<String>,
    },

    /// `all` spans several outputs, which cannot be combined yet
    #[error(
        "capturing all outputs as one image is not implemented yet; \
         choose one of {} with --output NAME or capture.output",
        .available.join(", ")
    )]
    CombinedCapture { available: Vec<String> },

    /// Creating the destination file failed
    #[error("failed to create PNG destination {path:?}")]
    CreateDestination {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Creating a generated parent directory failed
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

/// An empty output name
#[derive(Debug, thiserror::Error)]
#[error(r#"expected an output name, such as DP-1, or "all""#)]
pub struct EmptyOutputName;

/// A save path pattern that cannot prodce a PNG destination
#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    /// The pattern is empty
    #[error("expected a nonempty path pattern")]
    Empty,

    /// The pattern contains a NUL character, which paths cannot hold
    #[error("path patterns cannot contain NUL characters")]
    Nul,

    /// The pattern starts with `~/` but no home directory is available
    #[error("a leading ~/ requires HOME to be an absolute path")]
    MissingHome,

    /// The pattern contains an invalid strftime directive
    #[error(transparent)]
    Format(#[from] jiff::Error),

    /// The expanded path names a directory rather than a file
    #[error("expanded path {path:?} does not end with a filename")]
    MissingFilename { path: PathBuf },

    /// The expanded filename lacks a `.png` extension
    #[error("expanded path {path:?} is not a .png file")]
    NotPng { path: PathBuf },
}
