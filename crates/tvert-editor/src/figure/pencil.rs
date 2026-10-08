use crate::{figure::Figure, viewport::Viewport};
use iced::{
    Point,
    widget::canvas::{LineCap, LineJoin, Path, path::Builder},
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

    fn trace(&self, builder: &mut Builder, viewport: Viewport) {
        let Some((&start, rest)) = self.points.split_first() else {
            return;
        };
        builder.move_to(viewport.canvas_position(start));

        let Some((&next, _)) = rest.split_first() else {
            return;
        };
        builder.line_to(viewport.canvas_position(midpoint(start, next)));

        for pair in rest.windows(2) {
            if let &[control, next] = pair {
                builder.quadratic_curve_to(
                    viewport.canvas_position(control),
                    viewport.canvas_position(midpoint(control, next)),
                );
            }
        }

        if let Some(&end) = rest.last() {
            builder.line_to(viewport.canvas_position(end));
        }
    }
}

impl Figure for PencilFigure {
    fn path(&self, viewport: crate::viewport::Viewport) -> iced::widget::canvas::Path {
        Path::new(|builder| self.trace(builder, viewport))
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

const fn midpoint(first: Point, second: Point) -> Point {
    Point::new(first.x.midpoint(second.x), first.y.midpoint(second.y))
}
