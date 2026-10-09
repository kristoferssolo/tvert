use crate::figure::{Figure, RectangleFigure};
use iced::{Rectangle, widget::canvas::Fill};

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

    fn draw(
        &self,
        frame: &mut iced::widget::canvas::Frame,
        viewport: crate::viewport::Viewport,
        stroke: iced::widget::canvas::Stroke<'_>,
    ) {
        frame.fill(
            &self.path(viewport),
            Fill {
                style: stroke.style,
                ..Default::default()
            },
        );
    }
}
