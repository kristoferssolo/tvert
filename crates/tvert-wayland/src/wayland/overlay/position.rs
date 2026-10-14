use std::fmt::Display;

/// A finite position relative to an overlay surface's top-left corner.
///
/// Coordinates retain fractional precision and may lie outside the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfacePosition {
    x: f64,
    y: f64,
}

impl SurfacePosition {
    /// Creates a position, returning `None` if either coordinate is not finite
    pub const fn new(x: f64, y: f64) -> Option<Self> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }

        Some(Self { x, y })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }
}

impl Display for SurfacePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;
    use rstest::rstest;

    #[rstest]
    #[case(f64::NAN, 0.0)]
    #[case(0.0, f64::INFINITY)]
    fn rejects_non_finite_coordinates(#[case] x: f64, #[case] y: f64) {
        assert_none!(SurfacePosition::new(x, y));
    }
}
