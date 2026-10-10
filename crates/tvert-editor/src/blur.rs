use crate::{
    annotation::{AnnotationDraft, InvalidAnnotation},
    effect_region::EffectRegion,
    pixel_effect::PixelEffect,
};
use std::num::NonZeroU32;

/// A retained box-blur region and its radius in captured-image pixels
#[derive(Debug)]
pub struct Blur {
    region: EffectRegion,
    radius: NonZeroU32,
}

impl PixelEffect for Blur {
    fn region(&self) -> &EffectRegion {
        &self.region
    }

    fn apply(&self, image: &mut tvert_image::CapturedImage) -> Result<(), crate::Error> {
        let (x, y) = self.region.pixel_ranges(image.size())?;
        image.blur(x, y, self.radius)?;
        Ok(())
    }
}

impl TryFrom<&AnnotationDraft> for Blur {
    type Error = InvalidAnnotation;

    fn try_from(draft: &AnnotationDraft) -> Result<Self, Self::Error> {
        Ok(Self {
            region: EffectRegion::try_from(draft)?,
            radius: NonZeroU32::new(8).ok_or(InvalidAnnotation)?,
        })
    }
}
