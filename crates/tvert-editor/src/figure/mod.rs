mod arrow;
mod ellipse;
mod line;
mod rectangle;

pub use arrow::ArrowFigure;
pub use ellipse::EllipseFigure;
pub use line::LineFigure;
pub use rectangle::RectangleFigure;

use crate::{stroke_width::StrokeWidth, viewport::Viewport};
use iced::widget::canvas::Path;
use std::fmt::Debug;

/// An outline annotation stored in captured-image pixel coordinates
pub trait Figure: Debug {
    /// Builds the outline path for the current image viewport
    fn path(&self, viewport: Viewport) -> Path;
}

/// A stored figure and the stroke width chosen when its drag began
#[derive(Debug)]
pub struct Annotation {
    /// The annotation geometry in captured-image coordinates
    pub figure: Box<dyn Figure>,
    /// The width retained by this annotation
    pub stroke_width: StrokeWidth,
}
