use crate::{
    Error, ThemePreference, Tool,
    export::ExportBindings,
    size::{SizeKind, Sizes},
};
use iced::Color;
use strum::VariantArray;

/// Initial drawing settings for an editor session
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Preferred color scheme for the editor window
    pub theme: ThemePreference,
    /// Tool selected when the editor opens
    pub tool: Tool,
    /// Initial color for drawing strokes
    pub stroke_color: Color,
    /// Stroke width in image pixels
    pub stroke_width: u16,
    /// Pixelation block size in image pixels
    pub pixelation_block_size: u16,
    /// Blur sigma in image pixels
    pub blur_sigma: u16,
    /// Shortcuts for copying and saving
    pub export_bindings: ExportBindings,
}

impl Settings {
    /// Checks that each size is within its supported slider range and that
    /// export shortcuts are distinct and unused by other editor shortcuts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSetting`] for the first unsupported size, or
    /// [`Error::DuplicateKeyBinding`] or [`Error::ReservedKeyBinding`] for the
    /// first conflicting export shortcut.
    pub fn validate(self) -> Result<(), Error> {
        let sizes = Sizes::from(self);
        for &kind in SizeKind::VARIANTS {
            let (value, range) = (sizes.get(kind), kind.range());
            if !range.contains(&value) {
                return Err(Error::InvalidSetting {
                    name: kind.setting_name(),
                    value,
                    min: *range.start(),
                    max: *range.end(),
                });
            }
        }

        self.export_bindings.validate()
    }
}
