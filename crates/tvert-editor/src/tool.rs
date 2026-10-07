use crate::{
    figure::{EllipseFigure, Figure, LineFigure, RectangleFigure},
    viewport::rectangle_between,
};
use iced::Point;
use std::fmt::Display;

/// The figure created by the next drag
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Rectangle,
    Ellipse,
    Line,
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
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
        }
    }
}

impl Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
    fn accepts_lines_without_rectangle_area(#[case] end: Point) {
        let start = Point::new(10.0, 10.0);
        assert_some!(Tool::Line.figure(start, end));
        assert_none!(Tool::Rectangle.figure(start, end));
        assert_none!(Tool::Ellipse.figure(start, end));
    }

    #[rstest]
    #[case(Point::ORIGIN, Point::ORIGIN)]
    #[case(Point::new(f32::NAN, 10.0), Point::new(20.0, 20.0))]
    #[case(Point::new(10.0, 10.0), Point::new(f32::INFINITY, 20.0))]
    fn rejects_unusable_endpoints(#[case] start: Point, #[case] end: Point) {
        for tool in [Tool::Rectangle, Tool::Ellipse, Tool::Line] {
            assert_none!(tool.figure(start, end));
        }
    }
}
