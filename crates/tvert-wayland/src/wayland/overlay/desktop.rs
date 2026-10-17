use crate::{
    LogicalRect,
    wayland::overlay::{SurfacePosition, size::SurfaceSize},
};

/// A finite position in the compositor's logical desktop coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopPosition {
    x: f64,
    y: f64,
}

impl DesktopPosition {
    /// Maps a surface position through its configured size and output placement.
    ///
    /// Output placement already accounts for scale and rotation. Coordinates
    /// outside the surface are preserved for pointer motion during a grab.
    /// Scaling before dividing keeps pointer coordinates exact when the surface
    /// and output extents match, so [`Self::to_surface`] restores them.
    /// Returns `None` if either mapped coordinate is not finite.
    pub fn from_surface(
        position: SurfacePosition,
        surface: SurfaceSize,
        output: LogicalRect,
    ) -> Option<Self> {
        let x = position.x() * f64::from(output.width.get()) / f64::from(surface.width())
            + f64::from(output.x);
        let y = position.y() * f64::from(output.height.get()) / f64::from(surface.height())
            + f64::from(output.y);

        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        Some(Self { x, y })
    }
    /// Maps this position onto a surface covering `output`.
    ///
    /// Inverts [`Self::from_surface`]. Positions outside the output map
    /// outside the surface. Returns `None` if either coordinate is not finite.
    pub fn to_surface(self, surface: SurfaceSize, output: LogicalRect) -> Option<SurfacePosition> {
        let x = (self.x - f64::from(output.x)) * f64::from(surface.width())
            / f64::from(output.width.get());
        let y = (self.y - f64::from(output.y)) * f64::from(surface.height())
            / f64::from(output.height.get());
        SurfacePosition::new(x, y)
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::SuggestedSurfaceSize;
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use rstest::rstest;
    use std::num::NonZeroU32;

    fn placement(surface: (u32, u32), output: (i32, i32, u32, u32)) -> (SurfaceSize, LogicalRect) {
        let surface = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: surface.0,
            height: surface.1
        }));
        let output = LogicalRect {
            x: output.0,
            y: output.1,
            width: assert_some!(NonZeroU32::new(output.2)),
            height: assert_some!(NonZeroU32::new(output.3)),
        };
        (surface, output)
    }

    fn map(
        surface: (u32, u32),
        output: (i32, i32, u32, u32),
        point: (f64, f64),
    ) -> Option<DesktopPosition> {
        let (surface, output) = placement(surface, output);
        let point = assert_some!(SurfacePosition::new(point.0, point.1));
        DesktopPosition::from_surface(point, surface, output)
    }

    #[rstest]
    #[case::normal((100, 80), (0, 0, 100, 80), (12.5, 20.25), (12.5, 20.25))]
    #[case::negative_origin(
        (100, 80), (-1920, -200, 100, 80), (12.5, 20.25), (-1907.5, -179.75)
    )]
    #[case::different_extents((100, 80), (-1600, 120, 200, 160), (25.0, 20.0), (-1550.0, 160.0))]
    #[case::outside_surface((100, 80), (-100, 0, 100, 80), (150.0, -20.0), (50.0, -20.0))]
    #[case::fractional((200, 160), (10, -20, 100, 80), (0.5, 1.5), (10.25, -19.25))]
    #[case::portrait((160, 200), (0, 0, 80, 100), (20.0, 60.0), (10.0, 30.0))]
    #[case::left_of_negative_origin(
        (1920, 1080), (-1920, -1080, 1920, 1080), (1919.75, -0.5), (-0.25, -1080.5)
    )]
    fn maps_surface_positions_to_desktop_and_back(
        #[case] surface: (u32, u32),
        #[case] output: (i32, i32, u32, u32),
        #[case] point: (f64, f64),
        #[case] expected: (f64, f64),
    ) {
        let (surface, output) = placement(surface, output);
        let point = assert_some!(SurfacePosition::new(point.0, point.1));
        let mapped = assert_some!(DesktopPosition::from_surface(point, surface, output));
        assert_eq!(mapped.x().to_bits(), expected.0.to_bits());
        assert_eq!(mapped.y().to_bits(), expected.1.to_bits());
        assert_some_eq!(mapped.to_surface(surface, output), point);
    }

    #[test]
    fn unplaced_surfaces_keep_surface_coordinates() {
        let (surface, _) = placement((1920, 1080), (0, 0, 1, 1));
        let point = assert_some!(SurfacePosition::new(-12.5, 1100.25));
        assert_some_eq!(
            DesktopPosition::from_surface(point, surface, surface.at_origin()),
            DesktopPosition {
                x: -12.5,
                y: 1100.25
            }
        );
    }

    #[test]
    fn agrees_across_adjacent_outputs_with_different_surface_extents() {
        let from_left = assert_some!(map((100, 80), (-200, 0, 200, 160), (125.0, 40.0)));
        let from_right = assert_some!(map((200, 160), (0, 0, 100, 80), (100.0, 160.0)));
        assert_eq!(from_left, from_right);
    }

    #[rstest]
    #[case(f64::MAX, 0.0)]
    #[case(-f64::MAX, 0.0)]
    #[case(0.0, f64::MAX)]
    #[case(0.0, -f64::MAX)]
    fn rejects_overflowing_desktop_coordinates(#[case] x: f64, #[case] y: f64) {
        assert_none!(map((1, 1), (0, 0, 2, 2), (x, y)));
    }
}
