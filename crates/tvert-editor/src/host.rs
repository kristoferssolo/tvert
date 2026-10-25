use crate::{
    Error, Settings,
    editor::{Editor, ExportHandler, Message, shortcuts},
    toolbar::{HEIGHT, MIN_WIDTH},
};
use iced::{
    ContentFit, Element, Event, Size, Subscription, Task, Theme, event,
    keyboard::{Event as KeyboardEvent, Key, key::Named},
    widget::image::Handle,
    window,
};

#[derive(Debug, Clone)]
enum HostMessage {
    Editor(Message),
    Close,
}

pub fn run(
    preview: Handle,
    image_size: Size,
    settings: Settings,
    export: ExportHandler,
) -> Result<(), Error> {
    iced::application(
        move || Editor::new(preview.clone(), image_size, settings, export.clone()),
        update,
        view,
    )
    .theme(move |_: &Editor| Option::<Theme>::from(settings.theme))
    .subscription(move |_| {
        Subscription::batch([
            shortcuts::subscription(settings.export_bindings).map(HostMessage::Editor),
            event::listen_with(|event, status, _| close_on_escape(&event, status)),
        ])
    })
    .window(window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: "tvert".into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .window_size(initial_window_size(image_size))
    .resizable(true)
    .font(lucide_icons::LUCIDE_FONT_BYTES)
    .run()?;
    Ok(())
}

fn update(editor: &mut Editor, message: HostMessage) -> Task<HostMessage> {
    match message {
        HostMessage::Editor(message) => editor.update(message).map(HostMessage::Editor),
        HostMessage::Close => iced::exit(),
    }
}

fn view(editor: &Editor) -> Element<'_, HostMessage> {
    editor.view().map(HostMessage::Editor)
}

// Dismissal must also work while a widget owns the keyboard
fn close_on_escape(event: &Event, _status: event::Status) -> Option<HostMessage> {
    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Named(Named::Escape),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };
    modifiers.is_empty().then_some(HostMessage::Close)
}

fn initial_window_size(image_size: Size) -> Size {
    let available = Size::new(1080.0, 720.0 - HEIGHT);
    let fitted = ContentFit::Contain.fit(image_size, available);

    Size::new(fitted.width.max(MIN_WIDTH), fitted.height + HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;
    use iced::keyboard::{
        Location, Modifiers,
        key::{self},
    };

    fn press(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
        Event::Keyboard(KeyboardEvent::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::Escape),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat,
        })
    }

    #[test]
    fn escape_closes_even_when_a_control_or_gesture_captured_it() {
        for status in [event::Status::Ignored, event::Status::Captured] {
            assert!(matches!(
                close_on_escape(
                    &press(Key::Named(Named::Escape), Modifiers::empty(), false),
                    status,
                ),
                Some(HostMessage::Close),
            ));
        }
    }

    #[test]
    fn other_keys_modified_escape_and_repeat_do_not_close_the_host() {
        let events = [
            press(Key::Character("s".into()), Modifiers::CTRL, false),
            press(Key::Named(Named::Escape), Modifiers::CTRL, false),
            press(Key::Named(Named::Escape), Modifiers::SHIFT, false),
            press(Key::Named(Named::Escape), Modifiers::empty(), true),
        ];
        for event in events {
            assert_none!(close_on_escape(&event, event::Status::Ignored));
        }
    }
}
