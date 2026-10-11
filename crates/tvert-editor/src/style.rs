use iced::Color;
use std::ops::RangeInclusive;

pub const RED: Color = Color::from_rgb8(255, 64, 64);

pub const STROKE_WIDTH_RANGE: RangeInclusive<u16> = 1..=32;
pub const BLOCK_SIZE_RANGE: RangeInclusive<u16> = 4..=64;
pub const BLUR_SIGMA_RANGE: RangeInclusive<u16> = 4..=64;
