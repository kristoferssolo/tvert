use iced::Color;
use strum::{Display, VariantArray};

/// The initial color selected for annotation strokes
pub const DEFAULT_STROKE_COLOR: Color = Color::from_rgb8(255, 64, 64);

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

/// The side length of a pixelation block in captured-image pixels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, VariantArray)]
pub enum BlockSize {
    #[strum(serialize = "8 px")]
    Small,
    #[default]
    #[strum(serialize = "16 px")]
    Medium,
    #[strum(serialize = "32 px")]
    Large,
}

impl BlockSize {
    pub const fn pixels(self) -> u32 {
        match self {
            Self::Small => 8,
            Self::Medium => 16,
            Self::Large => 32,
        }
    }
}
