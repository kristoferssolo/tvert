use crate::{figure::Figure, stroke_width::StrokeWidth, tool::Tool, viewport::Viewport};
use iced::{
    Color, Point,
    widget::canvas::{Frame, Stroke},
};

/// A potential annotation described by a drag in captured-image coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationDraft {
    pub tool: Tool,
    pub stroke_width: StrokeWidth,
    pub start: Point,
    pub end: Point,
}

/// A stored figure and its retained appearance.
#[derive(Debug)]
pub struct Annotation {
    /// The annotation geometry in captured-image coordinates
    pub figure: Box<dyn Figure>,
    /// The width retained by this annotation
    pub stroke_width: StrokeWidth,
}

/// The selected tool cannot create geometry from the draft's endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("annotation draft has invalid or emtpy geometry")]
pub struct InvalidAnnotation;

impl Annotation {
    pub fn draw(&self, frame: &mut Frame, viewport: Viewport) {
        let stroke = Stroke::default()
            .with_color(Color::from_rgb8(255, 64, 64))
            .with_width(viewport.canvas_length(self.stroke_width.pixels()));

        frame.stroke(&self.figure.path(viewport), stroke);
    }
}

impl TryFrom<AnnotationDraft> for Annotation {
    type Error = InvalidAnnotation;

    fn try_from(draft: AnnotationDraft) -> Result<Self, Self::Error> {
        let figure = draft
            .tool
            .figure(draft.start, draft.end)
            .ok_or(InvalidAnnotation)?;

        Ok(Self {
            figure,
            stroke_width: draft.stroke_width,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some};
    use iced::{Size, widget::canvas::Path};
    use rstest::rstest;

    #[test]
    fn conversion_retains_width_and_endpoint_order() {
        let draft = AnnotationDraft {
            tool: Tool::Line,
            stroke_width: StrokeWidth::Thick,
            start: Point::new(150.0, 25.0),
            end: Point::new(50.0, 75.0),
        };
        let annotation = assert_ok!(Annotation::try_from(draft));
        let viewport = assert_some!(Viewport::new(
            Size::new(200.0, 100.0),
            Size::new(400.0, 400.0)
        ));
        let expected = Path::line(Point::new(300.0, 150.0), Point::new(100.0, 250.0));

        assert_eq!(annotation.stroke_width, StrokeWidth::Thick);
        assert_eq!(
            annotation
                .figure
                .path(viewport)
                .raw()
                .iter()
                .collect::<Vec<_>>(),
            expected.raw().iter().collect::<Vec<_>>()
        );
    }

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
            start,
            end
        }));

        assert_eq!(error, InvalidAnnotation);
    }
}
