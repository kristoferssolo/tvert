use crate::{Settings, Tool};
use std::ops::RangeInclusive;
use strum::VariantArray;

/// The adjustable size a tool draws with, in image pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq, VariantArray)]
pub enum SizeKind {
    StrokeWidth,
    BlockSize,
    BlurSigma,
}

impl SizeKind {
    pub const fn range(self) -> RangeInclusive<u16> {
        match self {
            Self::StrokeWidth => 1..=32,
            Self::BlockSize | Self::BlurSigma => 4..=64,
        }
    }

    /// The matching [`Settings`] field name
    pub const fn setting_name(self) -> &'static str {
        match self {
            Self::StrokeWidth => "stroke_width",
            Self::BlockSize => "pixelation_block_size",
            Self::BlurSigma => "blur_sigma",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::StrokeWidth => "Stroke width",
            Self::BlockSize => "Pixelation block size",
            Self::BlurSigma => "Blur strength",
        }
    }
}

impl Tool {
    pub(crate) const fn size_kind(self) -> SizeKind {
        match self {
            Self::Pixelate => SizeKind::BlockSize,
            Self::Blur => SizeKind::BlurSigma,
            _ => SizeKind::StrokeWidth,
        }
    }
}

/// One size value per [`SizeKind`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sizes {
    stroke_width: u16,
    block_size: u16,
    blur_sigma: u16,
}

impl Sizes {
    pub const fn get(self, kind: SizeKind) -> u16 {
        match kind {
            SizeKind::StrokeWidth => self.stroke_width,
            SizeKind::BlockSize => self.block_size,
            SizeKind::BlurSigma => self.blur_sigma,
        }
    }

    /// Stores `value` clamped to the kind's range
    pub fn set(&mut self, kind: SizeKind, value: u16) {
        let range = kind.range();
        let value = value.clamp(*range.start(), *range.end());
        match kind {
            SizeKind::StrokeWidth => self.stroke_width = value,
            SizeKind::BlockSize => self.block_size = value,
            SizeKind::BlurSigma => self.blur_sigma = value,
        }
    }
}

impl From<Settings> for Sizes {
    fn from(settings: Settings) -> Self {
        Self {
            stroke_width: settings.stroke_width,
            block_size: settings.pixelation_block_size,
            blur_sigma: settings.blur_sigma,
        }
    }
}
