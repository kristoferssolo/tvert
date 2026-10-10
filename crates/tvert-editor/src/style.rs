use iced::Color;
use std::ops::RangeInclusive;

pub const DEFAULT_STROKE_COLOR: Color = Color::from_rgb8(255, 64, 64);
pub const DEFAULT_STROKE_WIDTH: u16 = 4;
pub const DEFAULT_BLOCK_SIZE: u16 = 16;
pub const DEFAULT_BLUR_RADIUS: u16 = 8;

pub const STROKE_WIDTH_RANGE: RangeInclusive<u16> = 1..=32;
pub const BLOCK_SIZE_RANGE: RangeInclusive<u16> = 4..=64;
pub const BLUR_RADIUS_RANGE: RangeInclusive<u16> = 4..=64;
