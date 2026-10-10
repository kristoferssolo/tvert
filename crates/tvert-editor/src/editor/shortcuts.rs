use crate::{ExportAction, editor::message::Message};
use iced::{
    Event, event,
    keyboard::{Event as KeyboardEvent, Key},
};

/// Parses a history shortcut without changing the active draft
pub fn history(event: &Event) -> Option<Message> {
    match control_key(event)? {
        ('z', false) => Some(Message::Undo),
        ('z', true) | ('y', false) => Some(Message::Redo),
        _ => None,
    }
}

/// Parses export shortcuts only when widgets have not captured the event
pub fn export(event: &Event, status: event::Status) -> Option<Message> {
    if status != event::Status::Ignored {
        return None;
    }

    let action = match control_key(event)? {
        ('c', false) => ExportAction::Copy,
        ('c', true) => ExportAction::CopyAndSave,
        ('s', false) => ExportAction::Save,
        _ => return None,
    };

    Some(Message::Export(action))
}

fn control_key(event: &Event) -> Option<(char, bool)> {
    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Character(character),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };

    if !modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    let mut characters = character.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }

    Some((character.to_ascii_lowercase(), modifiers.shift()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;
    use iced::keyboard::{self, Modifiers, key};
    use rstest::rstest;

    fn key_press(character: &str, modifiers: Modifiers, repeat: bool) -> Event {
        Event::Keyboard(KeyboardEvent::KeyPressed {
            key: Key::Character(character.into()),
            modified_key: Key::Character(character.into()),
            physical_key: key::Physical::Code(key::Code::KeyZ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat,
        })
    }

    #[rstest]
    #[case("z", Modifiers::CTRL, Message::Undo)]
    #[case("Z", Modifiers::CTRL, Message::Undo)]
    #[case("y", Modifiers::CTRL, Message::Redo)]
    #[case("Y", Modifiers::CTRL, Message::Redo)]
    #[case("z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
    #[case("Z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
    fn parses_history(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: Message,
    ) {
        assert_eq!(
            history(&key_press(character, modifiers, false)),
            Some(expected)
        );
    }

    #[rstest]
    #[case("c", Modifiers::CTRL, ExportAction::Copy)]
    #[case("C", Modifiers::CTRL, ExportAction::Copy)]
    #[case("c", Modifiers::CTRL | Modifiers::SHIFT, ExportAction::CopyAndSave)]
    #[case("C", Modifiers::CTRL | Modifiers::SHIFT, ExportAction::CopyAndSave)]
    #[case("s", Modifiers::CTRL, ExportAction::Save)]
    #[case("S", Modifiers::CTRL, ExportAction::Save)]
    fn parses_export(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: ExportAction,
    ) {
        assert_eq!(
            export(
                &key_press(character, modifiers, false),
                event::Status::Ignored,
            ),
            Some(Message::Export(expected)),
        );
    }

    #[rstest]
    #[case(Modifiers::empty(), false)]
    #[case(Modifiers::CTRL | Modifiers::ALT, false)]
    #[case(Modifiers::CTRL | Modifiers::LOGO, false)]
    #[case(Modifiers::CTRL, true)]
    fn rejects_invalid_modifiers_and_repeats(#[case] modifiers: Modifiers, #[case] repeat: bool) {
        for character in ["z", "y", "c", "s"] {
            let event = key_press(character, modifiers, repeat);
            assert_none!(history(&event));
            assert_none!(export(&event, event::Status::Ignored));
        }
    }

    #[rstest]
    #[case("y", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case("s", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case("x", Modifiers::CTRL)]
    fn ignores_unbound_keys(#[case] character: &str, #[case] modifiers: Modifiers) {
        let event = key_press(character, modifiers, false);
        assert_none!(history(&event));
        assert_none!(export(&event, event::Status::Ignored));
    }

    #[test]
    fn ignores_captured_export_events() {
        assert_none!(export(
            &key_press("c", Modifiers::CTRL, false),
            event::Status::Captured,
        ));
    }
}
