use crate::{
    Error,
    annotation::{AnnotationDraft, InvalidAnnotation},
    pixel_effect::{EffectRegion, PixelEffect},
};
use std::num::NonZeroU32;
use tvert_image::CapturedImage;

/// A retained pixelation region measured in captured-image pixels
#[derive(Debug)]
pub struct Pixelation {
    region: EffectRegion,
    block_size: NonZeroU32,
}

impl PixelEffect for Pixelation {
    fn region(&self) -> &EffectRegion {
        &self.region
    }

    fn apply(&self, image: &mut CapturedImage) -> Result<(), Error> {
        let (x, y) = self.region.pixel_ranges(image.size())?;
        image.pixelate(x, y, self.block_size)?;
        Ok(())
    }
}

impl TryFrom<&AnnotationDraft> for Pixelation {
    type Error = InvalidAnnotation;

    fn try_from(draft: &AnnotationDraft) -> Result<Self, Self::Error> {
        Ok(Self {
            region: EffectRegion::try_from(draft)?,
            block_size: NonZeroU32::new(u32::from(draft.size)).ok_or(InvalidAnnotation)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotation::AnnotationDraft, tool::Tool};
    use claims::assert_ok;
    use iced::{Color, Point};
    use tvert_image::PixelSize;

    #[test]
    fn includes_fractional_edge_pixels_and_clips_to_the_image() {
        let effect = assert_ok!(Pixelation::try_from(&AnnotationDraft {
            tool: Tool::Pixelate,
            size: 16,
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

    #[test]
    fn applies_the_block_size_retained_by_the_draft() {
        let effect = assert_ok!(Pixelation::try_from(&AnnotationDraft {
            tool: Tool::Pixelate,
            size: 13,
            stroke_color: Color::BLACK,
            points: vec![Point::ORIGIN, Point::new(32.0, 1.0)],
        }));
        let size = assert_ok!(PixelSize::try_from((32, 1)));
        let rgba = (0..32).flat_map(|red| [red, 0, 0, 255]).collect();
        let mut image = assert_ok!(CapturedImage::new(size, rgba));

        assert_ok!(effect.apply(&mut image));

        let actual = image
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .step_by(8)
            .map(|&[red, _, _, _]| red)
            .collect::<Vec<_>>();
        assert_eq!(actual, [6, 6, 19, 19]);
    }
}
