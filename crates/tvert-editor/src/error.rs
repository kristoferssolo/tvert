/// An editor window failure
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Iced could not start or run the editor
    #[error("failed to run the editor")]
    Runtime(#[from] iced::Error),

    /// The preview does not contain decoded RGBA pixels
    #[error("editor preview does not contain raw RGBA pixels")]
    InvalidPreview,

    /// Image dimensions cannot produce an export viewport
    #[error("image dimensions cannot produce an export viewport")]
    InvalidViewport,

    /// Preparing the exported image failed
    #[error("failed to prepare the edited image")]
    Image(#[from] tvert_image::Error),

    /// The caller's save operation failed
    #[error("failed to save screenshot")]
    Save(#[source] Box<dyn std::error::Error + Send + Sync>),
}
