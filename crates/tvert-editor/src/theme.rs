use iced::Theme;
use strum::{EnumString, VariantNames};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, VariantNames)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum ThemePreference {
    /// Follow the system color scheme
    System,
    /// Use Iced's light theme
    Light,
    /// Use Iced's dark theme
    Dark,
}

impl From<ThemePreference> for Option<Theme> {
    fn from(preference: ThemePreference) -> Self {
        match preference {
            ThemePreference::System => None,
            ThemePreference::Light => Some(Theme::Light),
            ThemePreference::Dark => Some(Theme::Dark),
        }
    }
}
