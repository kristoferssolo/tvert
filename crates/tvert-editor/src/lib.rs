mod annotation;
mod annotation_canvas;
mod editor;
mod error;
mod figure;
mod message;
mod render;
mod stroke_width;
mod tool;
mod toolbar;
mod viewport;

pub use error::Error;

use crate::{
    editor::{Editor, SaveHandler},
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
/// Save invokes the caller's callback and closes the window only on success.
/// Save errors appear in the editor so the user can retry.
///
/// # Errors
///
/// Returns [`Error::Runtime`] if Iced cannot start or run the window.
pub fn run<E>(
    image: CapturedImage,
    save: impl Fn(&CapturedImage) -> Result<(), E> + 'static,
) -> Result<(), Error>
where
    E: StdError + Send + Sync + 'static,
{
    let (width, height) = image.size().into();
    let image_size = Size::from((width, height));
    let preview = Handle::from_rgba(width, height, image.into_rgba());
    let save: SaveHandler =
        Rc::new(move |image| save(image).map_err(|source| Error::Save(Box::new(source))));

    iced::application(
        move || Editor::new(preview.clone(), image_size, save.clone()),
        Editor::update,
        Editor::view,
    )
    .title("Tvert")
    .theme(Theme::Dark)
    .subscription(|_| event::listen_with(save_shortcut))
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

fn save_shortcut(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
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

    (modifiers.control()
        && !modifiers.shift()
        && !modifiers.alt()
        && !modifiers.logo()
        && character.eq_ignore_ascii_case("s"))
    .then_some(Message::Save)
}
