use crate::{ExportAction, Tool, editor::message::Message};
use iced::{
    Event, event,
    keyboard::{Event as KeyboardEvent, Key, Modifiers, key::Named},
};
use strum::VariantArray;

/// Switches to selecting existing edits
pub const SELECT_KEY: char = 'V';

pub fn parse(event: &Event, status: event::Status) -> Option<Message> {
    if status != event::Status::Ignored {
        return None;
    }
    history(event)
        .or_else(|| export(event))
        .or_else(|| tool(event))
        .or_else(|| selection(event))
}

pub fn history(event: &Event) -> Option<Message> {
    match control_key(event)? {
        ('z', false) => Some(Message::Undo),
        ('z', true) | ('y', false) => Some(Message::Redo),
        _ => None,
    }
}

pub fn tool(event: &Event) -> Option<Message> {
    let (character, modfiers) = character_key(event)?;
    if modfiers.control() || modfiers.alt() || modfiers.logo() {
        return None;
    }
    if SELECT_KEY.eq_ignore_ascii_case(&character) {
        return Some(Message::SelectionMode);
    }
    Tool::VARIANTS
        .iter()
        .copied()
        .find(|&tool| tool_key(tool).eq_ignore_ascii_case(&character))
        .map(Message::SelectTool)
}

pub const fn tool_key(tool: Tool) -> char {
    match tool {
        Tool::Rectangle => 'R',
        Tool::Ellipse => 'C',
        Tool::Line => 'L',
        Tool::Arrow => 'A',
        Tool::Pencil => 'P',
        Tool::Highlighter => 'H',
        Tool::Pixelate => 'X',
        Tool::Blur => 'B',
    }
}

fn selection(event: &Event) -> Option<Message> {
    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Named(named),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };

    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    match named {
        Named::Delete | Named::Backspace => Some(Message::DeleteSelected),
        Named::Escape => Some(Message::SelectEdit(None)),
        _ => None,
    }
}

fn export(event: &Event) -> Option<Message> {
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

fn character_key(event: &Event) -> Option<(char, Modifiers)> {
    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Character(character),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };

    let mut characters = character.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    Some((character, *modifiers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};
    use iced::keyboard::{self, key};
    use rstest::rstest;

    fn press(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
        Event::Keyboard(KeyboardEvent::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::KeyZ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat,
        })
    }

    fn key_press(character: &str, modifiers: Modifiers, repeat: bool) -> Event {
        press(Key::Character(character.into()), modifiers, repeat)
    }

    #[rstest]
    #[case("z", Modifiers::CTRL, Message::Undo)]
    #[case("y", Modifiers::CTRL, Message::Redo)]
    #[case("z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
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
    #[case("c", Modifiers::CTRL | Modifiers::SHIFT, ExportAction::CopyAndSave)]
    #[case("s", Modifiers::CTRL, ExportAction::Save)]
    fn parses_export(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: ExportAction,
    ) {
        assert_eq!(
            parse(
                &key_press(character, modifiers, false),
                event::Status::Ignored,
            ),
            Some(Message::Export(expected)),
        );
    }

    #[test]
    fn parses_the_key_shown_in_each_tooltip() {
        let bindings = Tool::VARIANTS
            .iter()
            .map(|&tool| (tool_key(tool), Message::SelectTool(tool)))
            .chain([(SELECT_KEY, Message::SelectionMode)]);

        for (character, expected) in bindings {
            let character = character.to_ascii_lowercase().to_string();
            assert_some_eq!(
                parse(
                    &key_press(&character, Modifiers::empty(), false),
                    event::Status::Ignored
                ),
                expected,
            );
        }
    }

    #[rstest]
    #[case("Z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
    #[case("R", Modifiers::SHIFT, Message::SelectTool(Tool::Rectangle))]
    fn ignores_character_case(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: Message,
    ) {
        assert_some_eq!(
            parse(
                &key_press(character, modifiers, false),
                event::Status::Ignored
            ),
            expected,
        );
    }

    #[rstest]
    #[case(Modifiers::CTRL, false)]
    #[case(Modifiers::ALT, false)]
    #[case(Modifiers::LOGO, false)]
    #[case(Modifiers::empty(), true)]
    fn ignores_modified_and_repeated_tool_keys(#[case] modifiers: Modifiers, #[case] repeat: bool) {
        for &variant in Tool::VARIANTS {
            let event = key_press(&tool_key(variant).to_string(), modifiers, repeat);
            assert_none!(tool(&event));
        }
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
            assert_none!(export(&event));
        }
    }

    #[rstest]
    #[case("y", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case("s", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case("x", Modifiers::CTRL)]
    fn ignores_unbound_keys(#[case] character: &str, #[case] modifiers: Modifiers) {
        let event = key_press(character, modifiers, false);
        assert_none!(history(&event));
        assert_none!(parse(&event, event::Status::Ignored));
    }

    #[rstest]
    #[case("z", Modifiers::CTRL)]
    #[case("r", Modifiers::empty())]
    fn ignores_captured_events(#[case] character: &str, #[case] modifiers: Modifiers) {
        assert_none!(parse(
            &key_press(character, modifiers, false),
            event::Status::Captured,
        ));
    }

    #[rstest]
    #[case(key::Named::Delete, Message::DeleteSelected)]
    #[case(key::Named::Backspace, Message::DeleteSelected)]
    #[case(key::Named::Escape, Message::SelectEdit(None))]
    fn acts_on_the_selection_only_with_uncaptured_keys(
        #[case] named: key::Named,
        #[case] expected: Message,
    ) {
        let key = press(Key::Named(named), Modifiers::empty(), false);
        assert_some_eq!(parse(&key, event::Status::Ignored), expected);
        assert_none!(parse(&key, event::Status::Captured));

        let modified = press(Key::Named(named), Modifiers::CTRL, false);
        assert_none!(parse(&modified, event::Status::Ignored));
    }
}
