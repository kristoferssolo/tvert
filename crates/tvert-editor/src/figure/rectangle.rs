use crate::{
    figure::Figure,
    viewport::{Viewport, translate_rectangle},
};
use iced::{Rectangle, widget::canvas::Path};
use std::fmt::Debug;

/// A rectangular outline in captured-image pixel coordinates
#[derive(Debug)]
pub struct RectangleFigure {
    bounds: Rectangle,
}

impl RectangleFigure {
    pub const fn new(bounds: Rectangle) -> Self {
        Self { bounds }
    }

    pub const fn bounds(&self) -> Rectangle {
        self.bounds
    }
}

impl From<Rectangle> for RectangleFigure {
    fn from(bounds: Rectangle) -> Self {
        Self::new(bounds)
    }
}

impl Figure for RectangleFigure {
    fn path(&self, viewport: Viewport) -> Path {
        let bounds = viewport.canvas_rectangle(self.bounds);
        Path::rectangle(bounds.position(), bounds.size())
    }

    fn translated(&self, offset: iced::Vector) -> Box<dyn Figure> {
        Box::new(Self::new(translate_rectangle(self.bounds, offset)))
    }
}
