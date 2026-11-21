mod annotation;
mod annotation_canvas;
mod color;
mod constraint;
mod crop;
mod edit;
mod editor;
mod error;
mod export;
mod figure;
mod guide;
mod hit;
mod host;
mod image_canvas;
mod key_bindings;
mod mode;
mod pixel_effect;
mod render;
mod settings;
mod size;
mod style;
mod theme;
mod tool;
mod toolbar;
mod viewport;

pub use color::parse_hex;
pub use error::Error;
pub use export::{ExportAction, ExportBindings};
pub use key_bindings::{KeyBinding, ParseKeyBindingError};
pub use settings::Settings;
pub use theme::ThemePreference;
pub use tool::Tool;

use crate::{editor::ExportHandler, render::handle_from_image};
use iced::Size;
use std::{error::Error as StdError, rc::Rc};
use tvert_image::CapturedImage;

/// Opens the captured image in a resizable editor window.
///
/// Takes ownership of the pixels and runs until the window closes.
/// Call this on the main thread after closing the selection overlay.
///
/// Export controls invoke the caller's callback with the requested action.
/// Successful exports close the window; export errors appear in the editor.
/// Escape closes it without exporting, including while a control has focus.
/// The caller must retain and serve clipboard ownership after this returns.
///
/// # Errors
///
/// Returns the [`Settings::validate`] error for unsupported settings, or
/// [`Error::Runtime`] if Iced cannot start or run the window.
pub fn run<E>(
    image: CapturedImage,
    settings: Settings,
    export: impl Fn(ExportAction, &CapturedImage) -> Result<(), E> + 'static,
) -> Result<(), Error>
where
    E: StdError + Send + Sync + 'static,
{
    settings.validate()?;

    let (width, height) = image.size().into();
    let image_size = Size::from((width, height));
    let preview = handle_from_image(image);
    let export: ExportHandler = Rc::new(move |action, image| {
        export(action, image).map_err(|source| Error::Export(Box::new(source)))
    });

    host::run(preview, image_size, settings, export)
}
