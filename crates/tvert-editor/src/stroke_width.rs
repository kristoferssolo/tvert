use strum::{Display, VariantArray};

/// The width of an annotation stroke in captured-image pixels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, VariantArray)]
pub enum StrokeWidth {
    /// A two-pixel stroke
    #[strum(serialize = "2 px")]
    Thin,
    #[default]
    /// A four-pixel stroke
    #[strum(serialize = "4 px")]
    Medium,
    /// A eight-pixel stroke
    #[strum(serialize = "8 px")]
    Thick,
}

impl StrokeWidth {
    pub const fn pixels(self) -> f32 {
        match self {
            Self::Thin => 2.0,
            Self::Medium => 4.0,
            Self::Thick => 8.0,
        }
    }
}
