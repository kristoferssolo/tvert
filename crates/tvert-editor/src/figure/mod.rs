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
use iced::{
    Vector,
    widget::canvas::{Fill, Frame, Path, Stroke},
};
use std::fmt::Debug;

/// A drawable annotation stored in captured-image pixel coordinates
pub trait Figure: Debug {
    /// Builds the figure path for the current image viewport
    fn path(&self, viewport: Viewport) -> Path;

    /// Returns a copy moved by `offset` captured-image pixels
    fn translated(&self, offset: Vector) -> Box<dyn Figure>;

    /// Whether the figure paints its interior instead of stroking its path.
    ///
    /// Drawing, hit testing, and selection outlines all follow this.
    fn filled(&self) -> bool {
        false
    }

    /// Draws the figure using its retained annotation appearance
    fn draw(&self, frame: &mut Frame, viewport: Viewport, stroke: Stroke<'_>) {
        let path = self.path(viewport);
        if self.filled() {
            frame.fill(
                &path,
                Fill {
                    style: stroke.style,
                    ..Default::default()
                },
            );
            return;
        }
        frame.stroke(&self.path(viewport), stroke);
    }
}
