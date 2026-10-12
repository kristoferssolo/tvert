use crate::{
    annotation::{AnnotationDraft, InvalidAnnotation},
    pixel_effect::{EffectRegion, PixelEffect},
};

/// A retained Gaussian blur region and its sigma in captured-image pixels
#[derive(Debug)]
pub struct Blur {
    region: EffectRegion,
    sigma: f32,
}

impl PixelEffect for Blur {
    fn region(&self) -> &EffectRegion {
        &self.region
    }

    fn apply(&self, image: &mut tvert_image::CapturedImage) -> Result<(), crate::Error> {
        let (x, y) = self.region.pixel_ranges(image.size())?;
        image.blur(x, y, self.sigma)?;
        Ok(())
    }

    fn translated(&self, offset: iced::Vector) -> Box<dyn PixelEffect> {
        Box::new(Self {
            region: self.region.translated(offset),
            ..*self
        })
    }
}

impl TryFrom<&AnnotationDraft> for Blur {
    type Error = InvalidAnnotation;

    fn try_from(draft: &AnnotationDraft) -> Result<Self, Self::Error> {
        if draft.size == 0 {
            return Err(InvalidAnnotation);
        }

        Ok(Self {
            region: EffectRegion::try_from(draft)?,
            sigma: f32::from(draft.size),
        })
    }
}
