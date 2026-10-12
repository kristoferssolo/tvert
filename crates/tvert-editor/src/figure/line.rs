use crate::figure::Figure;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::Viewport;
    use claims::assert_some;
    use iced::{Rectangle, Size};

    #[test]
    fn preserves_endpoint_order_when_mapping_to_the_canvas() {
        let viewport = assert_some!(Viewport::new(
            Size::new(200.0, 100.0),
            Rectangle::with_size(Size::new(400.0, 400.0))
        ));
        let figure = LineFigure::new(Point::new(150.0, 25.0), Point::new(50.0, 75.0));
        let path = figure.path(viewport);
        let expected = Path::line(Point::new(300.0, 150.0), Point::new(100.0, 250.0));

        assert_eq!(
            path.raw().iter().collect::<Vec<_>>(),
            expected.raw().iter().collect::<Vec<_>>(),
        );
    }
}
