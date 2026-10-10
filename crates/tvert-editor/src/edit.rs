use crate::{
    annotation::{Annotation, AnnotationDraft, InvalidAnnotation},
    blur::Blur,
    pixel_effect::PixelEffect,
    pixelation::Pixelation,
    tool::Tool,
    viewport::Viewport,
};
use iced::widget::canvas::Frame;

/// An ordered drawing or pixel effect in the editor's history
#[derive(Debug)]
pub enum Edit {
    Annotation(Annotation),
    Pixelation(Box<dyn PixelEffect>),
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
    pub fn draw_draft(&self, frame: &mut Frame, viewport: Viewport) {
        match self {
            Self::Annotation(annotation) => annotation.draw(frame, viewport),
            Self::Pixelation(effect) => effect.draw_selection(frame, viewport),
        }
    }
}

impl TryFrom<AnnotationDraft> for Edit {
    type Error = InvalidAnnotation;

    fn try_from(draft: AnnotationDraft) -> Result<Self, Self::Error> {
        match draft.tool {
            Tool::Pixelate => Ok(Self::Pixelation(Box::new(Pixelation::try_from(&draft)?))),
            Tool::Blur => Ok(Self::Pixelation(Box::new(Blur::try_from(&draft)?))),
            _ => Ok(Self::Annotation(Annotation::try_from(draft)?)),
        }
    }
}
