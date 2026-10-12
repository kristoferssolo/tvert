use crate::figure::{Figure, RectangleFigure};
use iced::Rectangle;

#[derive(Debug)]
pub struct HighlighterFigure {
    rectangle: RectangleFigure,
}

impl From<Rectangle> for HighlighterFigure {
    fn from(bounds: Rectangle) -> Self {
        Self {
            rectangle: bounds.into(),
        }
    }
}

impl Figure for HighlighterFigure {
    fn path(&self, viewport: crate::viewport::Viewport) -> iced::widget::canvas::Path {
        self.rectangle.path(viewport)
    }

    fn filled(&self) -> bool {
        true
    }
}
