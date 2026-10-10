use crate::{
    annotation::{Annotation, AnnotationDraft, InvalidAnnotation},
    pixel_effect::{Blur, PixelEffect, Pixelation},
    tool::Tool,
    viewport::Viewport,
};
use iced::widget::canvas::Frame;

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
