use crate::{
    Error,
    annotation::{AnnotationDraft, InvalidAnnotation},
    guide, hit,
    viewport::{Viewport, rectangle_between, translate_rectangle},
};
use iced::{
    Color, Point, Rectangle, Size, Vector,
    widget::canvas::{Frame, Path},
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
        guide::dashed(frame, &self.outline(viewport), Color::BLACK, Color::WHITE);
    }

    /// Whether a canvas `point` falls within the region as displayed
    pub fn is_hit(&self, viewport: Viewport, point: Point) -> bool {
        hit::fill(&self.outline(viewport), point)
    }

    pub fn canvas_bounds(&self, viewport: Viewport) -> Rectangle {
        viewport.canvas_rectangle(self.bounds)
    }

    pub const fn translated(&self, offset: Vector) -> Self {
        Self {
            bounds: translate_rectangle(self.bounds, offset),
        }
    }

    /// Limits `offset` so the moved region stays within an image of `size`
    pub fn constrain_offset(&self, offset: Vector, size: Size) -> Vector {
        Vector::new(
            shift(offset.x, self.bounds.x, self.bounds.width, size.width),
            shift(offset.y, self.bounds.y, self.bounds.height, size.height),
        )
    }

    fn outline(&self, viewport: Viewport) -> Path {
        let bounds = self.canvas_bounds(viewport);
        Path::rectangle(bounds.position(), bounds.size())
    }
}

/// Limits a one-axis `offset` to keep `start..start + length` within `0..extent`
fn shift(offset: f32, start: f32, length: f32, extent: f32) -> f32 {
    offset.min(extent - length - start).max(-start)
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
