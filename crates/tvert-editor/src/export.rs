use iced::advanced::graphics::core::keyboard;
use strum::VariantArray;

use crate::{Error, KeyBinding, editor::shortcuts};

/// The export operation requested by the user
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray)]
pub enum ExportAction {
    Copy,
    Save,
    CopyAndSave,
}

impl ExportAction {
    /// The matching [`ExportBindings`] field name
    #[must_use]
    pub const fn setting_name(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Save => "save",
            Self::CopyAndSave => "copy_and_save",
        }
    }
}

/// The shortcut for each [`ExportAction`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExportBindings {
    pub copy: KeyBinding,
    pub save: KeyBinding,
    pub copy_and_save: KeyBinding,
}

impl ExportBindings {
    #[must_use]
    pub const fn get(self, action: ExportAction) -> KeyBinding {
        match action {
            ExportAction::Copy => self.copy,
            ExportAction::Save => self.save,
            ExportAction::CopyAndSave => self.copy_and_save,
        }
    }

    /// Returns the action bound to a nonrepeated key press.
    pub(crate) fn action(self, event: &keyboard::Event) -> Option<ExportAction> {
        ExportAction::VARIANTS
            .iter()
            .copied()
            .find(|&action| self.get(action).matches(event))
    }

    pub(crate) fn validate(self) -> Result<(), Error> {
        for (index, &action) in ExportAction::VARIANTS.iter().enumerate() {
            let binding = self.get(action);
            if let Some(&first) = ExportAction::VARIANTS
                .iter()
                .take(index)
                .find(|&&other| self.get(other) == binding)
            {
                return Err(Error::DuplicateKeyBinding {
                    binding,
                    first,
                    second: action,
                });
            }
            if let Some(reserved) = shortcuts::reserved_by(binding) {
                return Err(Error::ReservedKeyBinding {
                    action,
                    binding,
                    reserved,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl ExportBindings {
    /// Parses bindings for tests, which cannot read the embedded configuration
    pub(crate) fn parse(copy: &str, save: &str, copy_and_save: &str) -> Self {
        use claims::assert_ok;

        Self {
            copy: assert_ok!(copy.parse()),
            save: assert_ok!(save.parse()),
            copy_and_save: assert_ok!(copy_and_save.parse()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use rstest::rstest;
    use std::assert_matches;

    #[rstest]
    #[case("Ctrl+R")]
    #[case("Alt+R")]
    #[case("Ctrl+Shift+Y")]
    fn accepts_keys_that_built_in_shortcuts_do_not_match(#[case] save: &str) {
        assert_ok!(ExportBindings::parse("Ctrl+C", save, "Ctrl+Shift+C").validate());
    }

    #[rstest]
    #[case("Ctrl+Z", "undo")]
    #[case("ctrl+shift+z", "redo")]
    #[case("Ctrl+Y", "redo")]
    #[case("r", "the Rectangle tool")]
    #[case("Shift+B", "the Blur tool")]
    #[case("V", "selection mode")]
    fn rejects_keys_that_built_in_shortcuts_match(#[case] save: &str, #[case] expected: &str) {
        let error = assert_err!(ExportBindings::parse("Ctrl+C", save, "Ctrl+Shift+C").validate());
        assert_matches!(
            error,
            Error::ReservedKeyBinding {
                action: ExportAction::Save,
                reserved,
                ..
            } if reserved == expected
        );
    }

    #[test]
    fn rejects_bindings_that_normalize_to_the_same_shortcut() {
        let bindings = ExportBindings::parse("Ctrl+C", "Ctrl+S", " CTRL + s ");
        assert_matches!(
            assert_err!(bindings.validate()),
            Error::DuplicateKeyBinding {
                first: ExportAction::Save,
                second: ExportAction::CopyAndSave,
                ..
            }
        );
    }
}
