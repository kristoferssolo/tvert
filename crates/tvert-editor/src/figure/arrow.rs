use crate::figure::Figure;
use iced::{Point, Vector, widget::canvas::Path};

/// An arrow pointing from its start to its end in captured-image pixels
#[derive(Debug)]
pub struct ArrowFigure {
    start: Point,
    end: Point,
}

impl ArrowFigure {
    pub const fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    fn head(&self) -> Option<(Point, Point)> {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let length = dx.hypot(dy);

        if !length.is_finite() || length <= 0.0 {
            return None;
        }

        let direction = Vector::new(dx / length, dy / length);
        let head_length = 24.0_f32.min(length / 2.0);
        let half_width = head_length / 2.0;

        let base = Point::new(
            (-direction.x).mul_add(head_length, self.end.x),
            (-direction.y).mul_add(head_length, self.end.y),
        );

        Some((
            Point::new(
                (-direction.y).mul_add(half_width, base.x),
                direction.x.mul_add(half_width, base.y),
            ),
            Point::new(
                direction.y.mul_add(half_width, base.x),
                (-direction.x).mul_add(half_width, base.y),
            ),
        ))
    }
}

impl Figure for ArrowFigure {
    fn path(&self, viewport: crate::viewport::Viewport) -> iced::widget::canvas::Path {
        let Some((first_wing, second_wing)) = self.head() else {
            return Path::new(|_| {});
        };

        let start = viewport.canvas_position(self.start);
        let end = viewport.canvas_position(self.end);
        let first_wing = viewport.canvas_position(first_wing);
        let second_wing = viewport.canvas_position(second_wing);

        Path::new(|builder| {
            builder.move_to(start);
            builder.line_to(end);
            builder.move_to(first_wing);
            builder.line_to(end);
            builder.line_to(second_wing);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::Viewport;
    use claims::{assert_none, assert_some};
    use iced::{Rectangle, Size};
    use rstest::rstest;

    #[rstest]
    #[case(
        Point::new(50.0, 50.0),
        Point::new(150.0, 50.0),
        Point::new(126.0, 62.0),
        Point::new(126.0, 38.0)
    )]
    #[case(
        Point::new(150.0, 50.0),
        Point::new(50.0, 50.0),
        Point::new(74.0, 38.0),
        Point::new(74.0, 62.0)
    )]
    #[case(
        Point::new(50.0, 50.0),
        Point::new(50.0, 150.0),
        Point::new(38.0, 126.0),
        Point::new(62.0, 126.0)
    )]
    #[case(
        Point::new(50.0, 150.0),
        Point::new(50.0, 50.0),
        Point::new(62.0, 74.0),
        Point::new(38.0, 74.0)
    )]
    fn places_the_head_behind_the_tip(
        #[case] start: Point,
        #[case] end: Point,
        #[case] first_wing: Point,
        #[case] second_wing: Point,
    ) {
        assert_eq!(
            ArrowFigure::new(start, end).head(),
            Some((first_wing, second_wing)),
        );
    }

    #[test]
    fn shrinks_the_head_for_short_arrows() {
        let arrow = ArrowFigure::new(Point::new(50.0, 50.0), Point::new(58.0, 50.0));

        assert_eq!(
            arrow.head(),
            Some((Point::new(54.0, 52.0), Point::new(54.0, 48.0))),
        );
    }

    #[test]
    fn maps_the_shaft_and_head_without_reversing_the_arrow() {
        let viewport = assert_some!(Viewport::new(
            Size::new(200.0, 100.0),
            Rectangle::with_size(Size::new(400.0, 400.0))
        ));
        let arrow = ArrowFigure::new(Point::new(150.0, 25.0), Point::new(50.0, 25.0));

        let expected = Path::new(|builder| {
            builder.move_to(Point::new(300.0, 150.0));
            builder.line_to(Point::new(100.0, 150.0));
            builder.move_to(Point::new(148.0, 126.0));
            builder.line_to(Point::new(100.0, 150.0));
            builder.line_to(Point::new(148.0, 174.0));
        });

        assert_eq!(
            arrow.path(viewport).raw().iter().collect::<Vec<_>>(),
            expected.raw().iter().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn rejects_a_head_without_direction() {
        assert_none!(ArrowFigure::new(Point::ORIGIN, Point::ORIGIN).head());
    }
}
