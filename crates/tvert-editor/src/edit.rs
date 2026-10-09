use crate::{
    annotation::{Annotation, AnnotationDraft, InvalidAnnotation},
    pixelation::Pixelation,
    tool::Tool,
    viewport::Viewport,
};
use iced::{Color, widget::canvas::Frame};

/// An ordered drawing or pixel effect in the editor's history
#[derive(Debug)]
pub enum Edit {
    Annotation(Annotation),
    Pixelation(Pixelation),
}

impl Edit {
    /// Returns the retained drawing, if present
    pub const fn annotation(&self) -> Option<&Annotation> {
        match self {
            Self::Annotation(annotation) => Some(annotation),
            Self::Pixelation(_) => None,
        }
    }

    /// Draws an in-progress annotation or pixel-effect selection outline
    pub fn draw_draft(&self, frame: &mut Frame, viewport: Viewport, selection_color: Color) {
        match self {
            Self::Annotation(annotation) => annotation.draw(frame, viewport),
            Self::Pixelation(effect) => effect.draw_selection(frame, viewport, selection_color),
        }
    }
}

impl TryFrom<AnnotationDraft> for Edit {
    type Error = InvalidAnnotation;

    fn try_from(draft: AnnotationDraft) -> Result<Self, Self::Error> {
        match draft.tool {
            Tool::Pixelate => Ok(Self::Pixelation(Pixelation::try_from(&draft)?)),
            _ => Ok(Self::Annotation(Annotation::try_from(draft)?)),
        }
    }
}
