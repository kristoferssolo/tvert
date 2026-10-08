mod annotation;
mod annotation_canvas;
mod editor;
mod error;
mod export;
mod figure;
mod message;
mod render;
mod stroke_width;
mod tool;
mod toolbar;
mod viewport;

pub use error::Error;
pub use export::ExportAction;

use crate::{
    editor::{Editor, ExportHandler},
    message::Message,
};
use iced::{
    Event, Size, Theme, event,
    keyboard::{Event as KeyboardEvent, Key},
    widget::image::Handle,
    window,
};
use std::{error::Error as StdError, rc::Rc};
use tvert_image::CapturedImage;

/// Opens an editor window containing the captured image.
///
/// Takes ownership of the pixels and runs until the window closes.
/// Call this on the main thread after closing the selection overlay.
///
/// Copy and Save invoke the caller's callback with the requested action.
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
    .subscription(|_| event::listen_with(export_shortcut))
    .window(window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: "tvert".into(),
            ..Default::default()
        },
        min_size: Some(Size::new(640.0, 320.0)),
        ..Default::default()
    })
    .window_size((1000.0, 700.0))
    .resizable(true)
    .run()?;
    Ok(())
}

fn export_shortcut(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    if status != event::Status::Ignored {
        return None;
    }

    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Character(character),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };

    if !modifiers.control() || modifiers.shift() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    let action = if character.eq_ignore_ascii_case("c") {
        ExportAction::Copy
    } else if character.eq_ignore_ascii_case("s") {
        ExportAction::Save
    } else {
        return None;
    };

    Some(Message::Export(action))
}
