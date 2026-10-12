use crate::{
    annotation::{Annotation, AnnotationDraft, InvalidAnnotation},
    pixel_effect::{Blur, PixelEffect, Pixelation},
    tool::Tool,
    viewport::Viewport,
};
use iced::{Point, Rectangle, widget::canvas::Frame};

/// An ordered drawing or pixel effect in the editor's history
#[derive(Debug)]
pub enum Edit {
    Annotation(Annotation),
    PixelEffect(Box<dyn PixelEffect>),
}

impl Edit {
    /// Returns the retained drawing, if present
    pub const fn annotation(&self) -> Option<&Annotation> {
        match self {
            Self::Annotation(annotation) => Some(annotation),
            Self::PixelEffect(_) => None,
        }
    }

    /// Draws an in-progress annotation or pixel-effect selection outline
    pub fn draw_draft(&self, frame: &mut Frame, viewport: Viewport) {
        match self {
            Self::Annotation(annotation) => annotation.draw(frame, viewport),
            Self::PixelEffect(effect) => effect.draw_selection(frame, viewport),
        }
    }

    /// Whether a canvas `point` touches the edit as displayed
    pub fn is_hit(&self, viewport: Viewport, point: Point) -> bool {
        match self {
            Self::Annotation(annotation) => annotation.is_hit(viewport, point),
            Self::PixelEffect(effect) => effect.region().is_hit(viewport, point),
        }
    }

    /// Returns the displayed extent in canvas coordinates
    pub fn canvas_bounds(&self, viewport: Viewport) -> Option<Rectangle> {
        match self {
            Self::Annotation(annotation) => annotation.canvas_bounds(viewport),
            Self::PixelEffect(effect) => Some(effect.region().canvas_bounds(viewport)),
        }
    }
}

/// Returns the index of the last-rendered edit under a canvas `point`
pub fn topmost_hit(edits: &[Edit], viewport: Viewport, point: Point) -> Option<usize> {
    edits.iter().rposition(|edit| edit.is_hit(viewport, point))
}

impl TryFrom<AnnotationDraft> for Edit {
    type Error = InvalidAnnotation;

    fn try_from(draft: AnnotationDraft) -> Result<Self, Self::Error> {
        match draft.tool {
            Tool::Pixelate => Ok(Self::PixelEffect(Box::new(Pixelation::try_from(&draft)?))),
            Tool::Blur => Ok(Self::PixelEffect(Box::new(Blur::try_from(&draft)?))),
            _ => Ok(Self::Annotation(Annotation::try_from(draft)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hit::TOLERANCE, style::RED};
    use claims::{assert_ok, assert_some};
    use iced::Size;
    use rstest::rstest;

    const IMAGE: Size = Size::new(200.0, 100.0);

    fn edit(tool: Tool, points: &[(f32, f32)]) -> Edit {
        assert_ok!(Edit::try_from(AnnotationDraft {
            tool,
            size: 4,
            stroke_color: RED,
            points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        }))
    }

    fn viewport(scale: f32) -> Viewport {
        assert_some!(Viewport::new(
            IMAGE,
            Rectangle::with_size(Size::new(IMAGE.width * scale, IMAGE.height * scale))
        ))
    }

    #[rstest]
    #[case::line_stroke(Tool::Line, &[(20.0, 50.0), (180.0, 50.0)], (100.0, 57.0), true)]
    #[case::line_beyond_reach(Tool::Line, &[(20.0, 50.0), (180.0, 50.0)], (100.0, 59.0), false)]
    #[case::arrowhead(Tool::Arrow, &[(20.0, 50.0), (180.0, 50.0)], (156.0, 69.0), true)]
    #[case::beside_arrowhead(Tool::Arrow, &[(20.0, 50.0), (180.0, 50.0)], (156.0, 72.0), false)]
    #[case::pencil_curve(Tool::Pencil, &[(20.0, 20.0), (100.0, 80.0), (180.0, 20.0)], (100.0, 70.0), true)]
    #[case::pencil_sample(Tool::Pencil, &[(20.0, 20.0), (100.0, 80.0), (180.0, 20.0)], (100.0, 80.0), false)]
    #[case::rectangle_edge(Tool::Rectangle, &[(20.0, 20.0), (180.0, 80.0)], (22.0, 50.0), true)]
    #[case::rectangle_interior(Tool::Rectangle, &[(20.0, 20.0), (180.0, 80.0)], (100.0, 50.0), false)]
    #[case::ellipse_interior(Tool::Ellipse, &[(20.0, 20.0), (180.0, 80.0)], (100.0, 50.0), false)]
    #[case::highlight_interior(Tool::Highlighter, &[(20.0, 20.0), (180.0, 80.0)], (100.0, 50.0), true)]
    #[case::effect_interior(Tool::Pixelate, &[(20.0, 20.0), (180.0, 80.0)], (100.0, 50.0), true)]
    #[case::beside_effect(Tool::Blur, &[(20.0, 20.0), (180.0, 80.0)], (100.0, 87.0), false)]
    fn hits_visible_geometry(
        #[case] tool: Tool,
        #[case] points: &[(f32, f32)],
        #[case] (x, y): (f32, f32),
        #[case] expected: bool,
    ) {
        assert_eq!(
            edit(tool, points).is_hit(viewport(1.0), Point::new(x, y)),
            expected
        );
    }

    #[rstest]
    #[case((100.0, 50.0), Some(2))]
    #[case((30.0, 50.0), Some(1))]
    #[case((30.0, 25.0), Some(0))]
    #[case((100.0, 95.0), None)]
    fn selects_the_topmost_overlapping_edit(
        #[case] (x, y): (f32, f32),
        #[case] expected: Option<usize>,
    ) {
        let edits = [
            edit(Tool::Pixelate, &[(20.0, 20.0), (180.0, 80.0)]),
            edit(Tool::Line, &[(20.0, 50.0), (180.0, 50.0)]),
            edit(Tool::Highlighter, &[(60.0, 30.0), (140.0, 70.0)]),
        ];

        assert_eq!(
            topmost_hit(&edits, viewport(1.0), Point::new(x, y)),
            expected
        );
    }

    #[rstest]
    fn measures_reach_in_viewport_pixels(#[values(0.25, 1.0, 4.0)] scale: f32) {
        let viewport = viewport(scale);
        let line = edit(Tool::Line, &[(20.0, 50.0), (180.0, 50.0)]);
        let reach = viewport.canvas_length(2.0) + TOLERANCE;
        let center = viewport.canvas_position(Point::new(100.0, 50.0));

        assert!(line.is_hit(viewport, Point::new(center.x, center.y + reach - 0.5)));
        assert!(!line.is_hit(viewport, Point::new(center.x, center.y + reach + 0.5)));
    }
}
