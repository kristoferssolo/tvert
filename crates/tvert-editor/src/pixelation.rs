use crate::{
    Error,
    annotation::{AnnotationDraft, InvalidAnnotation},
    viewport::{Viewport, rectangle_between},
};
use iced::{
    Color, Rectangle,
    widget::canvas::{Frame, LineDash, Path, Stroke},
};
use num_traits::ToPrimitive;
use std::num::NonZeroU32;
use tvert_image::CapturedImage;

/// A retained pixelation region measured in captured-image pixels
#[derive(Debug)]
pub struct Pixelation {
    bounds: Rectangle,
    block_size: NonZeroU32,
}

impl Pixelation {
    /// Applies the effect to the image's current pixels.
    ///
    /// Fractional edges include every intersected pixel.
    ///
    /// # Errors
    ///
    /// Returns an error if the region cannot be converted to valid pixel ranges
    /// or pixelation fails.
    pub fn apply(&self, image: &mut CapturedImage) -> Result<(), Error> {
        let size = image.size();
        let left = pixel_edge(f64::from(self.bounds.x).floor(), size.width().get())?;
        let top = pixel_edge(f64::from(self.bounds.y).floor(), size.height().get())?;
        let right = pixel_edge(
            (f64::from(self.bounds.x) + f64::from(self.bounds.width)).ceil(),
            size.width().get(),
        )?;
        let bottom = pixel_edge(
            (f64::from(self.bounds.y) + f64::from(self.bounds.height)).ceil(),
            size.height().get(),
        )?;

        image.pixelate(left..right, top..bottom, self.block_size)?;
        Ok(())
    }

    /// Draws the temporary region outline without applying the pixel effect
    pub fn draw_selection(&self, frame: &mut Frame, viewport: Viewport) {
        let bounds = viewport.canvas_rectangle(self.bounds);
        let path = Path::rectangle(bounds.position(), bounds.size());
        let stroke = Stroke::default().with_width(1.0);
        frame.stroke(&path, stroke.with_color(Color::BLACK));
        frame.stroke(
            &path,
            Stroke {
                line_dash: LineDash {
                    segments: &[6.0, 6.0],
                    offset: 0,
                },
                ..stroke.with_color(Color::WHITE)
            },
        );
    }
}

fn pixel_edge(value: f64, extent: u32) -> Result<usize, Error> {
    value
        .clamp(0.0, f64::from(extent))
        .to_usize()
        .ok_or(Error::InvalidViewport)
}

impl TryFrom<&AnnotationDraft> for Pixelation {
    type Error = InvalidAnnotation;

    fn try_from(draft: &AnnotationDraft) -> Result<Self, Self::Error> {
        let (&start, &end) = draft
            .points
            .first()
            .zip(draft.points.last())
            .ok_or(InvalidAnnotation)?;

        if !draft
            .points
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
        {
            return Err(InvalidAnnotation);
        }

        Ok(Self {
            bounds: rectangle_between(start, end).ok_or(InvalidAnnotation)?,
            block_size: NonZeroU32::new(16).ok_or(InvalidAnnotation)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotation::AnnotationDraft, stroke_width::StrokeWidth, tool::Tool};
    use claims::assert_ok;
    use iced::Point;
    use tvert_image::PixelSize;

    #[test]
    fn includes_fractional_edge_pixels_and_clips_to_the_image() {
        let effect = assert_ok!(Pixelation::try_from(&AnnotationDraft {
            tool: Tool::Pixelate,
            stroke_width: StrokeWidth::Medium,
            stroke_color: Color::BLACK,
            points: vec![Point::new(5.0, 2.0), Point::new(1.8, 0.2)],
        }));
        let size = assert_ok!(PixelSize::try_from((4, 1)));
        let mut image = assert_ok!(CapturedImage::new(
            size,
            vec![0, 0, 0, 255, 30, 0, 0, 255, 60, 0, 0, 255, 90, 0, 0, 255],
        ));

        assert_ok!(effect.apply(&mut image));

        assert_eq!(
            image.rgba(),
            [0, 0, 0, 255, 60, 0, 0, 255, 60, 0, 0, 255, 60, 0, 0, 255]
        );
    }
}
