use crate::{
    figure::{Figure, RectangleFigure},
    viewport::translate_rectangle,
};
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

    fn translated(&self, offset: iced::Vector) -> Box<dyn Figure> {
        Box::new(Self::from(translate_rectangle(
            self.rectangle.bounds(),
            offset,
        )))
    }

    fn filled(&self) -> bool {
        true
    }
}
