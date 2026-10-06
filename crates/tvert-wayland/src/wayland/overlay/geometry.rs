use crate::{
    Error,
    wayland::overlay::{SurfaceRect, size::SurfaceSize},
};
use std::ops::Range;
use tvert_image::PixelSize;
use wayland_client::protocol::wl_output::Transform;

/// Nonempty pixel ranges in the captured buffer, with exclusive upper bounds
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelRect {
    x: Range<usize>,
    y: Range<usize>,
}

impl PixelRect {
    pub const fn x(&self) -> &Range<usize> {
        &self.x
    }

    pub const fn y(&self) -> &Range<usize> {
        &self.y
    }

    pub fn from_surface(
        rectangle: SurfaceRect,
        surface: SurfaceSize,
        pixels: PixelSize,
        transform: Transform,
    ) -> Result<Option<Self>, Error> {
        let [left, top, right, bottom] = buffer_edges(rectangle, surface, pixels, transform)?;
        let x = pixel_range(left, right, pixels.width().get())?;
        let y = pixel_range(top, bottom, pixels.height().get())?;

        if x.is_empty() || y.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self { x, y }))
    }
}

pub fn buffer_edges(
    rectangle: SurfaceRect,
    surface: SurfaceSize,
    pixels: PixelSize,
    transform: Transform,
) -> Result<[f64; 4], Error> {
    let left = rectangle.left() / f64::from(surface.width());
    let top = rectangle.top() / f64::from(surface.height());
    let right = rectangle.right() / f64::from(surface.width());
    let bottom = rectangle.bottom() / f64::from(surface.height());

    let [left, top, right, bottom] = match transform {
        Transform::Normal => [left, top, right, bottom],
        Transform::_90 => [1.0 - bottom, left, 1.0 - top, right],
        Transform::_180 => [1.0 - right, 1.0 - bottom, 1.0 - left, 1.0 - top],
        Transform::_270 => [top, 1.0 - right, bottom, 1.0 - left],
        Transform::Flipped => [1.0 - right, top, 1.0 - left, bottom],
        Transform::Flipped90 => [top, left, bottom, right],
        Transform::Flipped180 => [left, 1.0 - bottom, right, 1.0 - top],
        Transform::Flipped270 => [1.0 - bottom, 1.0 - right, 1.0 - top, 1.0 - left],
        _ => return Err(Error::UnsupportedOutputTransform(u32::from(transform))),
    };

    let width = f64::from(pixels.width().get());
    let height = f64::from(pixels.height().get());

    Ok([left * width, top * height, right * width, bottom * height])
}

fn pixel_range(start: f64, end: f64, limit: u32) -> Result<Range<usize>, Error> {
    let upper_start = ceiling_edge(start, limit);
    let lower_start = if f64::from(upper_start) > start {
        upper_start.saturating_sub(1)
    } else {
        upper_start
    };

    let start = usize::try_from(lower_start).map_err(|_| Error::InvalidShmLayout)?;
    let end = usize::try_from(ceiling_edge(end, limit)).map_err(|_| Error::InvalidShmLayout)?;

    Ok(start..end)
}

// Find ceil(edge), clipped to the buffer extent, without a float-to-integer cast
fn ceiling_edge(edge: f64, limit: u32) -> u32 {
    let mut low = 0;
    let mut high = limit;

    while low < high {
        let middle = low.midpoint(high);

        if f64::from(middle) < edge {
            low = middle.saturating_add(1);
        } else {
            high = middle;
        }
    }

    low
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_ok, assert_some};
    use rstest::rstest;
    use std::num::NonZeroU32;

    fn bounds(start: (f64, f64), end: (f64, f64), transform: Transform) -> PixelRect {
        let surface = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 80,
            height: 40,
        }));
        let pixels = PixelSize::new(
            assert_some!(NonZeroU32::new(160)),
            assert_some!(NonZeroU32::new(80)),
        );
        let rectangle = assert_some!(SurfaceRect::from_position(
            assert_some!(SurfacePosition::new(start.0, start.1)),
            assert_some!(SurfacePosition::new(end.0, end.1)),
            surface,
        ));

        assert_some!(assert_ok!(PixelRect::from_surface(
            rectangle, surface, pixels, transform
        )))
    }

    #[rstest]
    #[case(Transform::Normal, 20..60, 10..40)]
    #[case(Transform::_90, 80..140, 10..30)]
    #[case(Transform::_180, 100..140, 40..70)]
    #[case(Transform::_270, 20..80, 50..70)]
    #[case(Transform::Flipped, 100..140, 10..40)]
    #[case(Transform::Flipped90, 20..80, 10..30)]
    #[case(Transform::Flipped180, 20..60, 40..70)]
    #[case(Transform::Flipped270, 80..140, 50..70)]
    fn maps_all_output_transforms(
        #[case] transform: Transform,
        #[case] x: Range<usize>,
        #[case] y: Range<usize>,
    ) {
        assert_eq!(
            bounds((10.0, 5.0), (30.0, 20.0), transform),
            PixelRect { x, y }
        );
    }

    #[rstest]
    #[case((10.25, 5.25), (30.25, 20.25), 20..61, 10..41)]
    #[case((10.1, 5.1), (10.2, 5.2), 20..21, 10..11)]
    #[case((-1.0, -1.0), (90.0, 50.0), 0..160, 0..80)]
    #[case((79.5, 39.5), (80.0, 40.0), 159..160, 79..80)]
    fn rounds_outward_within_buffer_bounds(
        #[case] start: (f64, f64),
        #[case] end: (f64, f64),
        #[case] x: Range<usize>,
        #[case] y: Range<usize>,
    ) {
        assert_eq!(bounds(start, end, Transform::Normal), PixelRect { x, y });
    }
}
