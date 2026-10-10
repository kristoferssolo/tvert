use strum::{Display, VariantArray};

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
