use crate::{figure::Figure, viewport::Viewport};
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
}

impl Figure for RectangleFigure {
    fn path(&self, viewport: Viewport) -> Path {
        let bounds = viewport.canvas_rectangle(self.bounds);
        Path::rectangle(bounds.position(), bounds.size())
    }
}
