use crate::{figure::Figure, viewport::translate};
use iced::{Point, widget::canvas::Path};

/// A line segment whose endpoints are captured-image pixel coordinates
#[derive(Debug)]
pub struct LineFigure {
    start: Point,
    end: Point,
}

impl LineFigure {
    pub const fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }
}

impl Figure for LineFigure {
    fn path(&self, viewport: crate::viewport::Viewport) -> iced::widget::canvas::Path {
        Path::line(
            viewport.canvas_position(self.start),
            viewport.canvas_position(self.end),
        )
    }

    fn translated(&self, offset: iced::Vector) -> Box<dyn Figure> {
        Box::new(Self::new(
            translate(self.start, offset),
            translate(self.end, offset),
        ))
    }
}
