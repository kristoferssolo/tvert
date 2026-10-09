mod annotation;
mod annotation_canvas;
mod edit;
mod editor;
mod error;
mod export;
mod figure;
mod message;
mod pixelation;
mod render;
mod shortcuts;
mod stroke_width;
mod tool;
mod toolbar;
mod viewport;

pub use error::Error;
pub use export::ExportAction;

use crate::{
    editor::{Editor, ExportHandler},
    toolbar::{HEIGHT, MIN_WIDTH},
};
use iced::{ContentFit, Size, Theme, event, widget::image::Handle, window};
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
/// Returns [`Error::Runtime`] if Iced cannot start or run the window.
pub fn run<E>(
    image: CapturedImage,
    export: impl Fn(ExportAction, &CapturedImage) -> Result<(), E> + 'static,
) -> Result<(), Error>
where
    E: StdError + Send + Sync + 'static,
{
    let (width, height) = image.size().into();
    let image_size = Size::from((width, height));
    let preview = Handle::from_rgba(width, height, image.into_rgba());
    let export: ExportHandler = Rc::new(move |action, image| {
        export(action, image).map_err(|source| Error::Export(Box::new(source)))
    });

    iced::application(
        move || Editor::new(preview.clone(), image_size, export.clone()),
        Editor::update,
        Editor::view,
    )
    .title("Tvert")
    .theme(Theme::Dark)
    .font(lucide_icons::LUCIDE_FONT_BYTES)
    .subscription(|_| event::listen_with(|event, status, _| shortcuts::export(&event, status)))
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
