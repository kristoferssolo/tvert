use crate::wayland::overlay::{SurfacePosition, size::SurfaceSize};

/// A nonempty rectangle clipped to surface-local bounds.
///
/// Edges are finite and satisfy `left < right` and `top < bottom`.
/// Right and bottom edges may equal the corresponding surface dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl SurfaceRect {
    /// Normalizes two positions and clips the result to the surface.
    ///
    /// Returns `None` if the clipped rectangle has no area.
    pub fn from_position(
        start: SurfacePosition,
        end: SurfacePosition,
        bounds: SurfaceSize,
    ) -> Option<Self> {
        let max_x = f64::from(bounds.width());
        let max_y = f64::from(bounds.height());

        let left = start.x().min(end.x()).clamp(0.0, max_x);
        let top = start.y().min(end.y()).clamp(0.0, max_y);
        let right = start.x().max(end.x()).clamp(0.0, max_x);
        let bottom = start.y().max(end.y()).clamp(0.0, max_y);

        if left >= right || top >= bottom {
            return None;
        }

        Some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    /// Returns the left edge
    pub const fn left(self) -> f64 {
        self.left
    }

    /// Returns the top edge
    pub const fn top(self) -> f64 {
        self.top
    }

    /// Returns the right edge
    pub const fn right(self) -> f64 {
        self.right
    }

    /// Returns the bottom edge
    pub const fn bottom(self) -> f64 {
        self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::SuggestedSurfaceSize;
    use claims::{assert_none, assert_ok, assert_some};
    use rstest::rstest;

    fn rectangle(start: (f64, f64), end: (f64, f64)) -> Option<SurfaceRect> {
        let bounds = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 100,
            height: 80
        }));

        let start = assert_some!(SurfacePosition::new(start.0, start.1));
        let end = assert_some!(SurfacePosition::new(end.0, end.1));

        SurfaceRect::from_position(start, end, bounds)
    }

    #[rstest]
    #[case((10.0, 20.0), (90.0, 70.0), [10.0, 20.0, 90.0, 70.0])]
    #[case((90.0, 70.0), (10.0, 20.0), [10.0, 20.0, 90.0, 70.0])]
    #[case((-10.0, -20.0), (120.0, 100.0), [0.0, 0.0, 100.0, 80.0])]
    #[case((12.25, 7.5), (50.75, 40.5), [12.25, 7.5, 50.75, 40.5])]
    fn normalizes_and_clips(
        #[case] start: (f64, f64),
        #[case] end: (f64, f64),
        #[case] expected: [f64; 4],
    ) {
        let rectangle = assert_some!(rectangle(start, end));
        let actual = [
            rectangle.left(),
            rectangle.top(),
            rectangle.right(),
            rectangle.bottom(),
        ];

        assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
    }

    #[rstest]
    #[case((20.0, 20.0), (20.0, 40.0))]
    #[case((10.0, 20.0), (90.0, 20.0))]
    #[case((-20.0, 10.0), (-1.0, 30.0))]
    #[case((101.0, 10.0), (120.0, 30.0))]
    #[case((10.0, -20.0), (30.0, -1.0))]
    #[case((10.0, 81.0), (30.0, 100.0))]
    fn rejects_empty_intersections(#[case] start: (f64, f64), #[case] end: (f64, f64)) {
        assert_none!(rectangle(start, end));
    }
}
