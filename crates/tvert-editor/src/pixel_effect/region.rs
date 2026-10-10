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
use std::ops::Range;
use tvert_image::PixelSize;

/// A retained selection shared by pixel effects
#[derive(Debug)]
pub struct EffectRegion {
    bounds: Rectangle,
}

impl EffectRegion {
    pub fn pixel_ranges(&self, size: PixelSize) -> Result<(Range<usize>, Range<usize>), Error> {
        let (width, height) = size.into();
        let left = pixel_edge(f64::from(self.bounds.x).floor(), width)?;
        let top = pixel_edge(f64::from(self.bounds.y).floor(), height)?;
        let right = pixel_edge(
            (f64::from(self.bounds.x) + f64::from(self.bounds.width)).ceil(),
            width,
        )?;
        let bottom = pixel_edge(
            (f64::from(self.bounds.y) + f64::from(self.bounds.height)).ceil(),
            height,
        )?;

        Ok((left..right, top..bottom))
    }

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

impl TryFrom<&AnnotationDraft> for EffectRegion {
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
        })
    }
}
