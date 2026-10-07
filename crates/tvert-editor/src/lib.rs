mod annotation_canvas;
mod editor;
mod figure;
mod message;
mod tool;
mod viewport;

use crate::editor::Editor;
use iced::{Size, Theme, widget::image::Handle, window};
use tvert_image::CapturedImage;

/// An editor window failure
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Iced could not start or run the editor
    #[error("failed to run the editor")]
    Runtime(#[from] iced::Error),
}

/// Opens an editor window containing the captured image.
///
/// Takes ownership of the pixels and runs until the window closes.
/// Call this on the main thread after closing the selection overlay.
///
/// # Errors
///
/// Returns [`Error::Runtime`] if Iced cannot start or run the window.
pub fn run(image: CapturedImage) -> Result<(), Error> {
    let size = image.size();
    let image_size = Size::from((size.width().get(), size.height().get()));
    let preview = Handle::from_rgba(size.width().get(), size.height().get(), image.into_rgba());

    iced::application(
        move || Editor::new(preview.clone(), image_size),
        Editor::update,
        Editor::view,
    )
    .title("Tvert")
    .theme(Theme::Dark)
    .window(window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: "tvert".into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .window_size((1000.0, 700.0))
    .resizable(true)
    .run()?;
    Ok(())
}
