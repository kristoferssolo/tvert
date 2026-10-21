use crate::{ExportAction, KeyBinding};

/// An editor window failure
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Iced could not start or run the editor
    #[error("failed to run the editor")]
    Runtime(#[from] iced::Error),

    #[error("editor preview does not contain raw RGBA pixels")]
    InvalidPreview,

    #[error("image dimensions cannot produce an export viewport")]
    InvalidViewport,

    #[error("failed to prepare the edited image")]
    Image(#[from] tvert_image::Error),

    /// The caller's export operation failed
    #[error("failed to export screenshot")]
    Export(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("{name} must be between {min} and {max}, got {value}")]
    InvalidSetting {
        name: &'static str,
        value: u16,
        min: u16,
        max: u16,
    },

    #[error(
        "export shortcuts {} and {} are both {binding}",
        .first.setting_name(),
        .second.setting_name()
    )]
    DuplicateKeyBinding {
        binding: KeyBinding,
        first: ExportAction,
        second: ExportAction,
    },

    /// A history, tool, or selection shortcut already handles the binding
    #[error(
        "export shortcut {} is {binding}, which is reserved for {reserved}",
        .action.setting_name()
    )]
    ReservedKeyBinding {
        action: ExportAction,
        binding: KeyBinding,
        /// The built-in editor action the binding would trigger
        reserved: String,
    },
}
