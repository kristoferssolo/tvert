use crate::Tool;
use iced::{Point, Rectangle, Vector};
use std::f32::consts::FRAC_1_SQRT_2;

/// Unit directions at every multiple of 45 degrees, in image coordinates
const OCTANTS: [Vector; 8] = [
    Vector::new(1.0, 0.0),
    Vector::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    Vector::new(0.0, 1.0),
    Vector::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    Vector::new(-1.0, 0.0),
    Vector::new(-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
    Vector::new(0.0, -1.0),
    Vector::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
];

/// How holding Shift limits the endpoint of a drag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    /// Equal width and height, so rectangles become squares and ellipses circles
    Square,
    /// Direction from the start snapped to the nearest multiple of 45 degrees
    Octant,
}

impl Constraint {
    pub const fn for_tool(tool: Tool) -> Option<Self> {
        match tool {
            Tool::Rectangle | Tool::Ellipse | Tool::Highlighter | Tool::Pixelate | Tool::Blur => {
                Some(Self::Square)
            }
            Tool::Line | Tool::Arrow => Some(Self::Octant),
            Tool::Pencil => None,
        }
    }

    /// Moves `end` so the drag from `start` satisfies the constraint.
    ///
    /// Near an edge of `area`, in image coordinates, the drag shortens along
    /// its constrained direction instead of being clamped per axis, so it
    /// keeps its shape. `start` must lie within `area`.
    pub fn apply(self, start: Point, end: Point, area: Rectangle) -> Point {
        let delta = Vector::new(end.x - start.x, end.y - start.y);
        let (direction, length) = match self {
            Self::Square => (
                Vector::new(delta.x.signum(), delta.y.signum()),
                delta.x.abs().max(delta.y.abs()),
            ),
            Self::Octant => OCTANTS
                .into_iter()
                .map(|direction| {
                    (
                        direction,
                        delta.x.mul_add(direction.x, delta.y * direction.y),
                    )
                })
                .max_by(|(_, left), (_, right)| left.total_cmp(right))
                .unwrap_or_default(),
        };
        let length = length
            .min(room(start.x - area.x, direction.x, area.width))
            .min(room(start.y - area.y, direction.y, area.height));

        Point::new(
            direction.x.mul_add(length, start.x),
            direction.y.mul_add(length, start.y),
        )
    }
}

/// Distance along `direction` from `start` before one axis leaves `0..=extent`
fn room(start: f32, direction: f32, extent: f32) -> f32 {
    match direction {
        ..0.0 => start / -direction,
        0.0 => f32::INFINITY,
        _ => (extent - start) / direction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some_eq};
    use iced::Size;
    use rstest::rstest;

    const IMAGE: Rectangle = Rectangle::new(Point::ORIGIN, Size::new(200.0, 100.0));

    #[rstest]
    #[case::wider(Point::new(40.0, 20.0), Point::new(40.0, 40.0))]
    #[case::taller_up_left(Point::new(5.0, 0.0), Point::new(0.0, 0.0))]
    #[case::shortened_by_left_edge(Point::new(0.0, 30.0), Point::new(0.0, 20.0))]
    fn squares_use_the_longer_side_within_the_image(#[case] end: Point, #[case] expected: Point) {
        let start = Point::new(10.0, 10.0);
        assert_eq!(Constraint::Square.apply(start, end, IMAGE), expected);
    }

    #[rstest]
    #[case::horizontal(Point::new(10.0, 10.0), Point::new(50.0, 12.0), Point::new(50.0, 10.0))]
    #[case::vertical_up(Point::new(10.0, 50.0), Point::new(13.0, 20.0), Point::new(10.0, 20.0))]
    #[case::diagonal(Point::new(10.0, 10.0), Point::new(30.0, 28.0), Point::new(29.0, 29.0))]
    #[case::shortened_by_right_edge(
        Point::new(190.0, 50.0),
        Point::new(200.0, 62.0),
        Point::new(200.0, 60.0)
    )]
    fn lines_snap_to_45_degrees_within_the_image(
        #[case] start: Point,
        #[case] end: Point,
        #[case] expected: Point,
    ) {
        let actual = Constraint::Octant.apply(start, end, IMAGE);
        assert!(
            actual.distance(expected) < 0.001,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn an_offset_area_shortens_the_drag_at_its_own_edge() {
        let crop = Rectangle::new(Point::new(50.0, 20.0), Size::new(100.0, 50.0));
        let start = Point::new(60.0, 30.0);
        assert_eq!(
            Constraint::Square.apply(start, Point::new(40.0, 40.0), crop),
            Point::new(50.0, 40.0)
        );
    }

    #[test]
    fn an_unmoved_drag_stays_empty() {
        let start = Point::new(10.0, 10.0);
        for constraint in [Constraint::Square, Constraint::Octant] {
            assert_eq!(constraint.apply(start, start, IMAGE), start);
        }
    }

    #[test]
    fn pencil_is_unconstrained() {
        assert_none!(Constraint::for_tool(Tool::Pencil));
        assert_some_eq!(Constraint::for_tool(Tool::Arrow), Constraint::Octant);
    }
}
