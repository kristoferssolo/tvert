use iced_core::Color;
use palette::{FromColor, Hsl, Srgb};

/// A selected color that retains hue and saturation at black and white.
///
/// Hue is expressed in degrees; saturation, lightness, and alpha are in `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub(crate) hue: f32,
    saturation: f32,
    lightness: f32,
    alpha: f32,
}

impl From<Color> for Selection {
    fn from(color: Color) -> Self {
        let hsl = Hsl::from_color(Srgb::new(
            channel(color.r),
            channel(color.g),
            channel(color.b),
        ));
        Self {
            hue: hsl.hue.into_positive_degrees(),
            saturation: hsl.saturation,
            lightness: hsl.lightness,
            alpha: channel(color.a),
        }
    }
}

impl From<Selection> for Color {
    fn from(selection: Selection) -> Self {
        let rgb = Srgb::from_color(Hsl::new(
            selection.hue,
            selection.saturation,
            selection.lightness,
        ));
        Self::from_rgba(rgb.red, rgb.green, rgb.blue, selection.alpha)
    }
}

impl Selection {
    /// Returns hue in degrees, in `0..360`
    #[must_use]
    pub const fn hue(self) -> f32 {
        self.hue
    }

    /// Returns HSL saturation, in `0..=`.
    #[must_use]
    pub const fn saturation(self) -> f32 {
        self.saturation
    }

    /// Returns HSL lightness, in `0..=1`
    #[must_use]
    pub const fn lightness(self) -> f32 {
        self.lightness
    }

    /// Replaces hue, wrapping finite values into `0..360` and ignoring non-finite values
    #[must_use]
    pub fn with_hue(mut self, hue: f32) -> Self {
        if hue.is_finite() {
            self.hue = hue.rem_euclid(360.0);
        }
        self
    }

    /// Replaces saturation, clamping finite values into `0..=1` and ignoring non-finite values
    #[must_use]
    pub const fn with_saturation(mut self, saturation: f32) -> Self {
        if saturation.is_finite() {
            self.saturation = saturation.clamp(0.0, 1.0);
        }
        self
    }

    /// Replaces lightness, clamping finite values into `0..=1` and ignoring non-finite values
    #[must_use]
    pub const fn with_lightness(mut self, lightness: f32) -> Self {
        if lightness.is_finite() {
            self.lightness = lightness.clamp(0.0, 1.0);
        }
        self
    }

    pub(crate) fn mix(self) -> (f32, f32) {
        let chroma = (1.0 - 2.0f32.mul_add(self.lightness, -1.0).abs()) * self.saturation;
        (chroma, self.lightness - chroma / 2.0)
    }

    pub(crate) fn with_mix(mut self, chroma: f32, white: f32) -> Self {
        self.lightness = (chroma / 2.0 + white).clamp(0.0, 1.0);
        let span = 1.0 - 2.0f32.mul_add(self.lightness, -1.0).abs();
        if span > f32::EPSILON {
            self.saturation = (chroma / span).clamp(0.0, 1.0);
        }
        self
    }
}

pub fn hue_color(hue: f32) -> Color {
    let rgb = Srgb::from_color(Hsl::new(hue, 1.0, 0.5));
    Color::from_rgb(rgb.red, rgb.green, rgb.blue)
}

const fn channel(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
