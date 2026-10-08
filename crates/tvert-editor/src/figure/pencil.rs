use crate::figure::Figure;
use iced::{
    Point,
    widget::canvas::{LineCap, LineJoin, Path},
};

/// A freehand stroke retaining its samples in captured-image pixels
#[derive(Debug)]
pub struct PencilFigure {
    points: Vec<Point>,
}

impl PencilFigure {
    pub const fn new(points: Vec<Point>) -> Self {
        Self { points }
    }
}

impl Figure for PencilFigure {
    fn path(&self, viewport: crate::viewport::Viewport) -> iced::widget::canvas::Path {
        Path::new(|builder| {
            if let Some((start, rest)) = self.points.split_first() {
                builder.move_to(viewport.canvas_position(*start));

                for &point in rest {
                    builder.line_to(viewport.canvas_position(point));
                }
            }
        })
    }

    fn draw(
        &self,
        frame: &mut iced::widget::canvas::Frame,
        viewport: crate::viewport::Viewport,
        stroke: iced::widget::canvas::Stroke<'_>,
    ) {
        frame.stroke(
            &self.path(viewport),
            stroke
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round),
        );
    }
}
