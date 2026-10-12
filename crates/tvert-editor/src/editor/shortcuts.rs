use crate::{ExportAction, Tool, editor::message::Message};
use iced::{
    Event, event,
    keyboard::{Event as KeyboardEvent, Key, Modifiers},
};
use strum::VariantArray;

pub fn parse(event: &Event, status: event::Status) -> Option<Message> {
    if status != event::Status::Ignored {
        return None;
    }
    history(event)
        .or_else(|| export(event))
        .or_else(|| tool(event))
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
            parse(
                &key_press(character, modifiers, false),
                event::Status::Ignored,
            ),
            Some(Message::Export(expected)),
        );
    }

    #[rstest]
    #[case(Tool::Rectangle, "r")]
    #[case(Tool::Ellipse, "c")]
    #[case(Tool::Line, "l")]
    #[case(Tool::Arrow, "a")]
    #[case(Tool::Pencil, "p")]
    #[case(Tool::Highlighter, "h")]
    #[case(Tool::Pixelate, "x")]
    #[case(Tool::Blur, "b")]
    fn parses_the_tool_key_shown_in_its_tooltip(
        #[case] tool: Tool,
        #[case] character: &str,
        #[values(Modifiers::empty(), Modifiers::SHIFT)] modifiers: Modifiers,
    ) {
        assert_eq!(tool_key(tool).to_string(), character.to_uppercase());

        for character in [character.to_lowercase(), character.to_uppercase()] {
            assert_some_eq!(
                parse(
                    &key_press(&character, modifiers, false),
                    event::Status::Ignored
                ),
                Message::SelectTool(tool),
            );
        }
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

    #[test]
    fn leaves_escape_to_widgets() {
        let escape = press(Key::Named(key::Named::Escape), Modifiers::empty(), false);
        assert_none!(parse(&escape, event::Status::Ignored));
    }

    #[rstest]
    #[case("z", Modifiers::CTRL)]
    #[case("c", Modifiers::CTRL)]
    #[case("s", Modifiers::CTRL)]
    #[case("e", Modifiers::empty())]
    fn ignores_captured_events(#[case] character: &str, #[case] modifiers: Modifiers) {
        assert_none!(parse(
            &key_press(character, modifiers, false),
            event::Status::Captured,
        ));
    }
}
