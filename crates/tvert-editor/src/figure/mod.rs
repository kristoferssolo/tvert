mod ellipse;
mod rectangle;

pub use ellipse::EllipseFigure;
pub use rectangle::RectangleFigure;

use crate::viewport::Viewport;
use iced::widget::canvas::Path;
use std::fmt::Debug;

/// An outline annotation stored in captured-image pixel coordinates
pub trait Figure: Debug {
    /// Builds the outline path for the current image viewport
    fn path(&self, viewport: Viewport) -> Path;
}
