use crate::{
    figure::{ArrowFigure, EllipseFigure, Figure, LineFigure, RectangleFigure},
    viewport::rectangle_between,
};
use iced::Point;
use strum::{Display, VariantArray};

/// The figure created by the next drag
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, VariantArray)]
pub enum Tool {
    Rectangle,
    Ellipse,
    Line,
    #[default]
    Arrow,
}

impl Tool {
    pub fn figure(self, start: Point, end: Point) -> Option<Box<dyn Figure>> {
        if ![start.x, start.y, end.x, end.y]
            .into_iter()
            .all(f32::is_finite)
            || start == end
        {
            return None;
        }
        Some(match self {
            Self::Rectangle => Box::new(RectangleFigure::new(rectangle_between(start, end)?)),
            Self::Ellipse => Box::new(EllipseFigure::new(rectangle_between(start, end)?)),
            Self::Line => Box::new(LineFigure::new(start, end)),
            Self::Arrow => Box::new(ArrowFigure::new(start, end)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use rstest::rstest;

    #[rstest]
    #[case(Point::new(50.0, 10.0))]
    #[case(Point::new(10.0, 50.0))]
    fn accepts_lines_and_arrows_without_rectangle_area(#[case] end: Point) {
        let start = Point::new(10.0, 10.0);

        assert_some!(Tool::Line.figure(start, end));
        assert_some!(Tool::Arrow.figure(start, end));
        assert_none!(Tool::Rectangle.figure(start, end));
        assert_none!(Tool::Ellipse.figure(start, end));
    }

    #[rstest]
    #[case(Point::ORIGIN, Point::ORIGIN)]
    #[case(Point::new(f32::NAN, 10.0), Point::new(20.0, 20.0))]
    #[case(Point::new(10.0, 10.0), Point::new(f32::INFINITY, 20.0))]
    fn rejects_unusable_endpoints(#[case] start: Point, #[case] end: Point) {
        for &tool in Tool::VARIANTS {
            assert_none!(tool.figure(start, end));
        }
    }
}
