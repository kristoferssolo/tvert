use crate::{KeyBinding, Tool, editor::message::Message, export::ExportBindings};
use iced::{
    Event, Subscription, event,
    keyboard::{Event as KeyboardEvent, Key, Modifiers, key::Named},
};
use strum::VariantArray;

/// Switches to selecting existing edits
pub const SELECT_KEY: char = 'V';

/// Publishes shortcut messages for keyboard events that no widget captured
pub fn subscription(exports: ExportBindings) -> Subscription<Message> {
    event::listen_with(|event, status, _| {
        matches!(event, Event::Keyboard(_)).then_some((event, status))
    })
    .with(exports)
    .filter_map(|(exports, (event, status))| parse(&event, status, exports))
}

pub fn parse(event: &Event, status: event::Status, exports: ExportBindings) -> Option<Message> {
    if status != event::Status::Ignored {
        return None;
    }
    built_in(event).or_else(|| match event {
        Event::Keyboard(event) => exports.action(event).map(Message::Export),
        _ => None,
    })
}

/// Names the history, tool, or selection action that already handles `binding`
pub fn reserved_by(binding: KeyBinding) -> Option<String> {
    let name = match built_in(&Event::Keyboard(binding.press()))? {
        Message::Undo => "undo".to_owned(),
        Message::Redo => "redo".to_owned(),
        Message::SelectTool(tool) => format!("the {tool} tool"),
        Message::SelectionMode => "selection mode".to_owned(),
        // Delete, Backspace, and Escape, which bindings cannot express
        _ => "the selection".to_owned(),
    };
    Some(name)
}

fn built_in(event: &Event) -> Option<Message> {
    history(event)
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
    let (character, modifiers) = character_key(event)?;
    if modifiers.control() || modifiers.alt() || modifiers.logo() {
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
    use crate::ExportAction;
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

    fn exports() -> ExportBindings {
        ExportBindings::parse("Ctrl+C", "Ctrl+S", "Ctrl+Shift+C")
    }

    #[rstest]
    #[case("z", Modifiers::CTRL, Message::Undo)]
    #[case("y", Modifiers::CTRL, Message::Redo)]
    fn parses_history(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: Message,
    ) {
        let event = key_press(character, modifiers, false);
        assert_some_eq!(parse(&event, event::Status::Ignored, exports()), expected);
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
                    event::Status::Ignored,
                    exports()
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
                event::Status::Ignored,
                exports()
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
        let event = key_press("r", modifiers, repeat);
        assert_none!(tool(&event));
    }

    #[rstest]
    #[case(Modifiers::empty(), false)]
    #[case(Modifiers::CTRL | Modifiers::ALT, false)]
    #[case(Modifiers::CTRL | Modifiers::LOGO, false)]
    #[case(Modifiers::CTRL, true)]
    fn rejects_invalid_control_keys(#[case] modifiers: Modifiers, #[case] repeat: bool) {
        let event = key_press("z", modifiers, repeat);
        assert_none!(control_key(&event));
    }

    #[rstest]
    #[case("y", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case("s", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case("x", Modifiers::CTRL)]
    fn ignores_unbound_keys(#[case] character: &str, #[case] modifiers: Modifiers) {
        let event = key_press(character, modifiers, false);
        assert_none!(parse(&event, event::Status::Ignored, exports()));
    }

    #[rstest]
    #[case("z", Modifiers::CTRL)]
    #[case("r", Modifiers::empty())]
    fn ignores_captured_events(#[case] character: &str, #[case] modifiers: Modifiers) {
        assert_none!(parse(
            &key_press(character, modifiers, false),
            event::Status::Captured,
            exports()
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
        assert_some_eq!(parse(&key, event::Status::Ignored, exports()), expected);
        assert_none!(parse(&key, event::Status::Captured, exports()));

        let modified = press(Key::Named(named), Modifiers::CTRL, false);
        assert_none!(parse(&modified, event::Status::Ignored, exports()));
    }

    #[rstest]
    #[case("1", Modifiers::ALT, Some(ExportAction::Copy))]
    #[case("X", Modifiers::CTRL | Modifiers::ALT, Some(ExportAction::Save))]
    #[case("s", Modifiers::LOGO, Some(ExportAction::CopyAndSave))]
    #[case("c", Modifiers::CTRL, None)]
    fn configured_bindings_replace_the_default_exports(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: Option<ExportAction>,
    ) {
        let exports = ExportBindings::parse("Alt+1", "ctrl+alt+x", "Super+S");
        assert_eq!(
            parse(
                &key_press(character, modifiers, false),
                event::Status::Ignored,
                exports,
            ),
            expected.map(Message::Export),
        );
    }

    #[test]
    fn exports_ignore_captured_and_repeated_presses() {
        let copy = |repeat| key_press("c", Modifiers::CTRL, repeat);

        assert_some_eq!(
            parse(&copy(false), event::Status::Ignored, exports()),
            Message::Export(ExportAction::Copy),
        );
        assert_none!(parse(&copy(false), event::Status::Captured, exports()));
        assert_none!(parse(&copy(true), event::Status::Ignored, exports()));
    }
}
