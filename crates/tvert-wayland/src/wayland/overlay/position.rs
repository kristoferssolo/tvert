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

    /// Returns the horizontal surface-local coordinate
    pub const fn x(self) -> f64 {
        self.x
    }

    /// Returns the vertical surface-local coordinate
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
    use claims::{assert_none, assert_some};
    use rstest::rstest;

    #[rstest]
    #[case(12.75, -3.125)]
    #[case(0.0, -0.0)]
    #[case(f64::MAX, -f64::MAX)]
    fn preserved_finite_coordinates(#[case] x: f64, #[case] y: f64) {
        let position = assert_some!(SurfacePosition::new(x, y));

        assert_eq!(position.x().to_bits(), x.to_bits());
        assert_eq!(position.y().to_bits(), y.to_bits());
    }

    #[rstest]
    #[case(f64::NAN, 0.0)]
    #[case(f64::INFINITY, 0.0)]
    #[case(f64::NEG_INFINITY, 0.0)]
    #[case(0.0, f64::NAN)]
    #[case(0.0, f64::INFINITY)]
    #[case(0.0, f64::NEG_INFINITY)]
    fn rejects_non_finite_coordinates(#[case] x: f64, #[case] y: f64) {
        assert_none!(SurfacePosition::new(x, y));
    }
}
