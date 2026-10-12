mod blur;
mod pixelation;
mod region;

pub use blur::Blur;
pub use pixelation::Pixelation;
pub use region::EffectRegion;

use crate::{Error, viewport::Viewport};
use iced::{Vector, widget::canvas::Frame};
use std::fmt::Debug;
use tvert_image::CapturedImage;

/// An image operation retained in the editor's ordered history
pub trait PixelEffect: Debug {
    /// Returns the selected region in captured-image coordinates
    fn region(&self) -> &EffectRegion;

    /// Applies the effect to the image's current pixels.
    ///
    /// # Errors
    ///
    /// Returns an error if mapping the region or processing its pixels fails.
    fn apply(&self, image: &mut CapturedImage) -> Result<(), Error>;

    /// Returns a copy whose region is moved by `offset` captured-image pixels
    fn translated(&self, offset: Vector) -> Box<dyn PixelEffect>;

    /// Draws the temporary region outline without applying the effect
    fn draw_selection(&self, frame: &mut Frame, viewport: Viewport) {
        self.region().draw_selection(frame, viewport);
    }
}
