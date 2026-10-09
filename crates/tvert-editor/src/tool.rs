use crate::{
    figure::{
        ArrowFigure, EllipseFigure, Figure, HighlighterFigure, LineFigure, PencilFigure,
        RectangleFigure,
    },
    viewport::rectangle_between,
};
use iced::Point;
use lucide_icons::Icon;
use strum::{Display, VariantArray};

/// The figure created by the next drag
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, VariantArray)]
pub enum Tool {
    Rectangle,
    Ellipse,
    Line,
    #[default]
    Arrow,
    Pencil,
    Highlighter,
}

impl Tool {
    pub fn figure(self, points: Vec<Point>) -> Option<Box<dyn Figure>> {
        let (&start, &end) = (points.first()?, points.last()?);

        if !points.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            return None;
        }

        let empty = match self {
            Self::Pencil => points.iter().all(|p| *p == start),
            _ => start == end,
        };

        if empty {
            return None;
        }

        let figure: Box<dyn Figure> = match self {
            Self::Rectangle => Box::new(RectangleFigure::from(rectangle_between(start, end)?)),
            Self::Ellipse => Box::new(EllipseFigure::from(rectangle_between(start, end)?)),
            Self::Line => Box::new(LineFigure::new(start, end)),
            Self::Arrow => Box::new(ArrowFigure::new(start, end)),
            Self::Pencil => Box::new(PencilFigure::new(points)),
            Self::Highlighter => Box::new(HighlighterFigure::from(rectangle_between(start, end)?)),
        };

        Some(figure)
    }

    pub const fn icon(self) -> Icon {
        match self {
            Self::Rectangle => Icon::Square,
            Self::Ellipse => Icon::Circle,
            Self::Line => Icon::Minus,
            Self::Arrow => Icon::ArrowRight,
            Self::Pencil => Icon::Pencil,
            Self::Highlighter => Icon::Highlighter,
        }
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

        assert_some!(Tool::Line.figure(vec![start, end]));
        assert_some!(Tool::Arrow.figure(vec![start, end]));
        assert_none!(Tool::Rectangle.figure(vec![start, end]));
        assert_none!(Tool::Ellipse.figure(vec![start, end]));
        assert_none!(Tool::Highlighter.figure(vec![start, end]));
    }

    #[rstest]
    #[case(Point::ORIGIN, Point::ORIGIN)]
    #[case(Point::new(f32::NAN, 10.0), Point::new(20.0, 20.0))]
    #[case(Point::new(10.0, 10.0), Point::new(f32::INFINITY, 20.0))]
    fn rejects_unusable_endpoints(#[case] start: Point, #[case] end: Point) {
        for &tool in Tool::VARIANTS {
            assert_none!(tool.figure(vec![start, end]));
        }
    }
}
