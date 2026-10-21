mod annotation;
mod annotation_canvas;
mod color;
mod edit;
mod editor;
mod error;
mod export;
mod figure;
mod guide;
mod hit;
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

use crate::{
    editor::{Editor, ExportHandler, shortcuts},
    render::handle_from_image,
    toolbar::{HEIGHT, MIN_WIDTH},
};
use iced::{ContentFit, Size, Theme, window};
use std::{error::Error as StdError, rc::Rc};
use tvert_image::CapturedImage;

/// Opens an editor window containing the captured image.
///
/// Takes ownership of the pixels and runs until the window closes.
/// Call this on the main thread after closing the selection overlay.
///
/// Export controls invoke the caller's callback with the requested action.
/// The window closes only on success; export errors appear in the editor.
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

    iced::application(
        move || Editor::new(preview.clone(), image_size, settings, export.clone()),
        Editor::update,
        Editor::view,
    )
    .title("Tvert")
    .theme(move |_: &Editor| Option::<Theme>::from(settings.theme))
    .font(lucide_icons::LUCIDE_FONT_BYTES)
    .subscription(move |_| shortcuts::subscription(settings.export_bindings))
    .window(window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: "tvert".into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .window_size(initial_window_size(image_size))
    .resizable(true)
    .run()?;
    Ok(())
}

fn initial_window_size(image_size: Size) -> Size {
    let available = Size::new(1080.0, 720.0 - HEIGHT);
    let fitted = ContentFit::Contain.fit(image_size, available);

    Size::new(fitted.width.max(MIN_WIDTH), fitted.height + HEIGHT)
}
