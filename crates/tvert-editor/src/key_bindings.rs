use iced::keyboard::{self, Key, Modifiers};
use std::{fmt::Display, str::FromStr};
use strum::{Display, EnumString, VariantArray};

/// A logical ASCII letter or digit with an exact set of keyboard modifiers.
///
/// Syntax is `Ctrl+Alt+Shift+Super+K`, with any subset of those modifiers.
/// Names and letters are case-insensitive; surrounding token whitespace is
/// ignored. Display uses that modifier order and an uppercase letter.
/// Named keys and punctuation are not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    key: char,
    modifiers: Modifiers,
}

/// An invalid keybinding string
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseKeyBindingError {
    #[error("missing key; expected an ASCII letter or digit")]
    MissingKey,
    #[error("unsupported key {0:?}; expected one ASCII letter or digit")]
    UnsupportedKey(String),
    #[error("unknown modifier {0:?}; expected Ctrl, Alt, Shift, or Super")]
    UnknownModifier(String),
    #[error("modifier {0:?} is repeated")]
    DuplicateModifier(String),
}

#[derive(Debug, Clone, Copy, Display, EnumString, VariantArray)]
#[strum(ascii_case_insensitive)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Super,
}

impl Modifier {
    const fn flags(self) -> Modifiers {
        match self {
            Self::Ctrl => Modifiers::CTRL,
            Self::Alt => Modifiers::ALT,
            Self::Shift => Modifiers::SHIFT,
            Self::Super => Modifiers::LOGO,
        }
    }
}

impl KeyBinding {
    /// Matches a nonrepeated logical key press, ignoring character case.
    ///
    /// Extra or missing modifiers prevent a match. The caller must discard
    /// captured events before calling this method.
    #[must_use]
    pub fn matches(self, event: &keyboard::Event) -> bool {
        let keyboard::Event::KeyPressed {
            key: Key::Character(key),
            modifiers,
            repeat: false,
            ..
        } = event
        else {
            return false;
        };
        *modifiers == self.modifiers && character(key) == Some(self.key)
    }
}

impl FromStr for KeyBinding {
    type Err = ParseKeyBindingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('+');
        let key = parts
            .next_back()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or(ParseKeyBindingError::MissingKey)?;
        let key =
            character(key).ok_or_else(|| ParseKeyBindingError::UnsupportedKey(key.to_owned()))?;
        let mut modifiers = Modifiers::empty();
        for part in parts {
            let part = part.trim();
            let modifier = part
                .parse::<Modifier>()
                .map_err(|_| ParseKeyBindingError::UnknownModifier(part.to_owned()))?
                .flags();

            if modifiers.contains(modifier) {
                return Err(ParseKeyBindingError::DuplicateModifier(part.to_owned()));
            }
            modifiers.insert(modifier);
        }
        Ok(Self { key, modifiers })
    }
}

impl Display for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for modifier in Modifier::VARIANTS {
            if self.modifiers.contains(modifier.flags()) {
                write!(f, "{modifier}+")?;
            }
        }
        write!(f, "{}", self.key.to_ascii_uppercase())
    }
}

fn character(value: &str) -> Option<char> {
    let mut characters = value.chars();
    let character = characters.next()?;
    if characters.next().is_some() || !character.is_ascii_alphanumeric() {
        return None;
    }
    Some(character.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use iced::keyboard::{Location, key};
    use rstest::rstest;

    fn press(key: Key, modifiers: Modifiers, repeat: bool) -> keyboard::Event {
        keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: key::Physical::Code(key::Code::KeyC),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat,
        }
    }

    #[rstest]
    #[case(" ctrl + SHIFT + c ", "Ctrl+Shift+C", "C", Modifiers::CTRL | Modifiers::SHIFT)]
    #[case(
        "super+alt+Ctrl+shift+s",
        "Ctrl+Alt+Shift+Super+S",
        "s",
        Modifiers::all()
    )]
    #[case("Alt+7", "Alt+7", "7", Modifiers::ALT)]
    #[case("v", "V", "V", Modifiers::empty())]
    fn parsed_bindings_match_their_canonical_labels(
        #[case] source: &str,
        #[case] label: &str,
        #[case] key: &str,
        #[case] modifiers: Modifiers,
    ) {
        let binding = assert_ok!(source.parse::<KeyBinding>());
        assert_eq!(binding.to_string(), label);
        assert_eq!(assert_ok!(label.parse::<KeyBinding>()), binding);
        assert!(binding.matches(&press(Key::Character(key.into()), modifiers, false)));
    }

    #[rstest]
    #[case("", ParseKeyBindingError::MissingKey)]
    #[case("Ctrl+", ParseKeyBindingError::MissingKey)]
    #[case("Ctrl+Enter", ParseKeyBindingError::UnsupportedKey("Enter".into()))]
    #[case("Ctrl+é", ParseKeyBindingError::UnsupportedKey("é".into()))]
    #[case("Ctrl++C", ParseKeyBindingError::UnknownModifier(String::new()))]
    #[case("Meta+C", ParseKeyBindingError::UnknownModifier("Meta".into()))]
    #[case("Ctrl+ctrl+C", ParseKeyBindingError::DuplicateModifier("ctrl".into()))]
    fn rejects_invalid_bindings(#[case] source: &str, #[case] expected: ParseKeyBindingError) {
        assert_eq!(assert_err!(source.parse::<KeyBinding>()), expected);
    }

    #[test]
    fn requires_an_exact_nonrepeated_character_press() {
        let binding = assert_ok!("Ctrl+C".parse::<KeyBinding>());
        let character = || Key::Character("c".into());
        let events = [
            press(character(), Modifiers::empty(), false),
            press(character(), Modifiers::CTRL | Modifiers::SHIFT, false),
            press(character(), Modifiers::CTRL | Modifiers::ALT, false),
            press(character(), Modifiers::CTRL | Modifiers::LOGO, false),
            press(character(), Modifiers::CTRL, true),
            press(Key::Character("s".into()), Modifiers::CTRL, false),
            press(Key::Character("cc".into()), Modifiers::CTRL, false),
            press(Key::Named(key::Named::Escape), Modifiers::CTRL, false),
            keyboard::Event::KeyReleased {
                key: character(),
                modified_key: character(),
                physical_key: key::Physical::Code(key::Code::KeyC),
                location: Location::Standard,
                modifiers: Modifiers::CTRL,
            },
        ];
        for event in events {
            assert!(!binding.matches(&event), "{event:?}");
        }
    }
}
