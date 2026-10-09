mod arrow;
mod ellipse;
mod highlighter;
mod line;
mod pencil;
mod rectangle;

pub use arrow::ArrowFigure;
pub use ellipse::EllipseFigure;
pub use highlighter::HighlighterFigure;
pub use line::LineFigure;
pub use pencil::PencilFigure;
pub use rectangle::RectangleFigure;

use crate::viewport::Viewport;
use iced::widget::canvas::{Frame, Path, Stroke};
use std::fmt::Debug;

/// A drawable annotation stored in captured-image pixel coordinates
pub trait Figure: Debug {
    /// Builds the figure path for the current image viewport
    fn path(&self, viewport: Viewport) -> Path;

    /// Draws the figure using its retained annotation appearance
    fn draw(&self, frame: &mut Frame, viewport: Viewport, stroke: Stroke<'_>) {
        frame.stroke(&self.path(viewport), stroke);
    }
}
