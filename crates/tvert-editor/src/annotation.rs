use crate::{figure::Figure, hit, tool::Tool, viewport::Viewport};
use iced::{
    Color, Point, Rectangle,
    widget::canvas::{Frame, Stroke},
};

const HIGHLIGHT_OPACITY: f32 = 0.3;

/// A potential annotation described by points in captured-image coordinates
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationDraft {
    pub tool: Tool,
    /// Stroke width, pixelation block side, or blur sigma in captured-image pixels
    pub size: u16,
    pub stroke_color: Color,
    pub points: Vec<Point>,
}

/// A stored figure and its retained appearance.
#[derive(Debug)]
pub struct Annotation {
    pub figure: Box<dyn Figure>,
    pub stroke_width: u16,
    pub stroke_color: Color,
}

/// The selected tool cannot create geometry from the draft's endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("annotation draft has invalid or emtpy geometry")]
pub struct InvalidAnnotation;

impl AnnotationDraft {
    pub fn new(tool: Tool, size: u16, stroke_color: Color, start: Point) -> Self {
        Self {
            tool,
            size,
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
            .with_width(viewport.canvas_length(f32::from(self.stroke_width)));

        self.figure.draw(frame, viewport, stroke);
    }

    /// Whether a canvas `point` touches the annotation as displayed
    pub fn is_hit(&self, viewport: Viewport, point: Point) -> bool {
        let path = self.figure.path(viewport);
        if self.figure.filled() {
            return hit::fill(&path, point);
        }
        hit::stroke(&path, point, self.canvas_width(viewport))
    }

    /// Returns the displayed extent in canvas coordinates, including the stroke
    pub fn canvas_bounds(&self, viewport: Viewport) -> Option<Rectangle> {
        let bounds = hit::extent(&self.figure.path(viewport))?;
        if self.figure.filled() {
            return Some(bounds);
        }
        Some(bounds.expand(self.canvas_width(viewport) / 2.0))
    }

    fn canvas_width(&self, viewport: Viewport) -> f32 {
        viewport.canvas_length(f32::from(self.stroke_width))
    }
}

impl TryFrom<AnnotationDraft> for Annotation {
    type Error = InvalidAnnotation;

    fn try_from(draft: AnnotationDraft) -> Result<Self, Self::Error> {
        let figure = draft.tool.figure(draft.points).ok_or(InvalidAnnotation)?;
        let stroke_color = match draft.tool {
            Tool::Highlighter => draft.stroke_color.scale_alpha(HIGHLIGHT_OPACITY),
            _ => draft.stroke_color,
        };

        Ok(Self {
            figure,
            stroke_width: draft.size,
            stroke_color,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::RED;
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
            size: 4,
            stroke_color: RED,
            points: vec![start, end]
        }));

        assert_eq!(error, InvalidAnnotation);
    }
}
