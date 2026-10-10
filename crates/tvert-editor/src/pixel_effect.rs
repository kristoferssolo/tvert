use crate::{Error, effect_region::EffectRegion, viewport::Viewport};
use iced::widget::canvas::Frame;
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

    /// Draws the temporary region outline without applying the effect
    fn draw_selection(&self, frame: &mut Frame, viewport: Viewport) {
        self.region().draw_selection(frame, viewport);
    }
}
