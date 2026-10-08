use crate::{figure::Figure, stroke_width::StrokeWidth, tool::Tool, viewport::Viewport};
use iced::{
    Color, Point,
    widget::canvas::{Frame, Stroke},
};

/// The initial color selected for annotation strokes
pub const DEFAULT_STROKE_COLOR: Color = Color::from_rgb8(255, 64, 64);

/// A potential annotation described by points in captured-image coordinates
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationDraft {
    pub tool: Tool,
    pub stroke_width: StrokeWidth,
    pub stroke_color: Color,
    pub points: Vec<Point>,
}

/// A stored figure and its retained appearance.
#[derive(Debug)]
pub struct Annotation {
    pub figure: Box<dyn Figure>,
    pub stroke_width: StrokeWidth,
    pub stroke_color: Color,
}

/// The selected tool cannot create geometry from the draft's endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("annotation draft has invalid or emtpy geometry")]
pub struct InvalidAnnotation;

impl AnnotationDraft {
    pub fn new(tool: Tool, stroke_width: StrokeWidth, stroke_color: Color, start: Point) -> Self {
        Self {
            tool,
            stroke_width,
            stroke_color,
            points: vec![start],
        }
    }

    /// Retains every pencil sample, or updates the endpoint of another tool
    pub fn record(&mut self, point: Point) {
        if self.points.last() == Some(&point) {
            return;
        }

        match self.points.as_mut_slice() {
            [_, .., end] if self.tool != Tool::Pencil => *end = point,
            _ => self.points.push(point),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.points
            .first()
            .is_none_or(|start| self.points.iter().all(|point| point == start))
    }
}

impl Annotation {
    pub fn draw(&self, frame: &mut Frame, viewport: Viewport) {
        let stroke = Stroke::default()
            .with_color(self.stroke_color)
            .with_width(viewport.canvas_length(self.stroke_width.pixels()));

        self.figure.draw(frame, viewport, stroke);
    }
}

impl TryFrom<AnnotationDraft> for Annotation {
    type Error = InvalidAnnotation;

    fn try_from(draft: AnnotationDraft) -> Result<Self, Self::Error> {
        let figure = draft.tool.figure(draft.points).ok_or(InvalidAnnotation)?;

        Ok(Self {
            figure,
            stroke_width: draft.stroke_width,
            stroke_color: draft.stroke_color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use rstest::rstest;

    #[rstest]
    #[case(Tool::Rectangle, Point::new(10.0, 10.0), Point::new(30.0, 10.0))]
    #[case(Tool::Ellipse, Point::new(10.0, 10.0), Point::new(10.0, 30.0))]
    #[case(Tool::Line, Point::ORIGIN, Point::ORIGIN)]
    #[case(Tool::Arrow, Point::new(f32::NAN, 10.0), Point::new(30.0, 20.0))]
    fn conversion_rejects_invalid_geometry(
        #[case] tool: Tool,
        #[case] start: Point,
        #[case] end: Point,
    ) {
        let error = assert_err!(Annotation::try_from(AnnotationDraft {
            tool,
            stroke_width: StrokeWidth::Medium,
            stroke_color: DEFAULT_STROKE_COLOR,
            points: vec![start, end]
        }));

        assert_eq!(error, InvalidAnnotation);
    }
}
