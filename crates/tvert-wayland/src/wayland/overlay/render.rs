use crate::{
    Error,
    wayland::overlay::{SurfaceRect, size::SurfaceSize},
};
use tvert_image::{CapturedImage, PixelSize};
use wayland_client::protocol::wl_output::Transform;

const BORDER_COLOR: [u8; 4] = [255, 64, 64, 255];
const BORDER_WIDTH: f64 = 2.0;

/// Renders an inward selection outline over a copy of the frozen image.
///
/// The rectangle is in surface coordinates. The returned image retains the
/// original buffer dimensions and orientation. Coverage uses pixel centers.
///
/// # Errors
///
/// Returns [`Error::UnsupportedOutputTransform`] for an unsupported transform,
/// [`Error::InvalidShmLayout`] if row addressing cannot be represented,
/// [`Error::ImageAllocation`] if allocating the copy fails, or [`Error::Image`]
/// if constructing the resulting image fails.
pub fn selection_outline(
    image: &CapturedImage,
    rectangle: SurfaceRect,
    surface_size: SurfaceSize,
    transform: Transform,
) -> Result<CapturedImage, Error> {
    let outline = Outline::new(rectangle, surface_size, image.size(), transform)?;
    let row_bytes = usize::try_from(image.size().width().get())
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or(Error::InvalidShmLayout)?;

    let mut rgba = Vec::new();
    rgba.try_reserve_exact(image.rgba().len())?;
    rgba.extend_from_slice(image.rgba());

    for (y, row) in rgba.chunks_exact_mut(row_bytes).enumerate() {
        let y = f64::from(u32::try_from(y).map_err(|_| Error::InvalidShmLayout)?) + 0.5;

        for (x, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let x = f64::from(u32::try_from(x).map_err(|_| Error::InvalidShmLayout)?) + 0.5;

            if outline.contains(x, y) {
                *pixel = BORDER_COLOR;
            }
        }
    }

    Ok(CapturedImage::new(image.size(), rgba)?)
}

struct Outline {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    stroke_x: f64,
    stroke_y: f64,
}

impl Outline {
    fn new(
        rectangle: SurfaceRect,
        surface: SurfaceSize,
        pixels: PixelSize,
        transform: Transform,
    ) -> Result<Self, Error> {
        let surface_width = f64::from(surface.width());
        let surface_height = f64::from(surface.height());
        let left = rectangle.left() / surface_width;
        let top = rectangle.top() / surface_height;
        let right = rectangle.right() / surface_width;
        let bottom = rectangle.bottom() / surface_height;

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

        let (axis_width, axis_height) = match transform {
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
                (surface_height, surface_width)
            }
            _ => (surface_width, surface_height),
        };

        let width = f64::from(pixels.width().get());
        let height = f64::from(pixels.height().get());

        Ok(Self {
            left: left * width,
            top: top * height,
            right: right * width,
            bottom: bottom * height,
            stroke_x: BORDER_WIDTH * width / axis_width,
            stroke_y: BORDER_WIDTH * height / axis_height,
        })
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        if x < self.left || x >= self.right || y < self.top || y >= self.bottom {
            return false;
        }

        x < self.left + self.stroke_x
            || x >= self.right - self.stroke_x
            || y < self.top + self.stroke_y
            || y >= self.bottom - self.stroke_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_ok, assert_some};
    use rstest::rstest;
    use std::{iter::repeat_n, num::NonZeroU32};

    fn pixels(width: u32, height: u32) -> PixelSize {
        PixelSize::new(
            assert_some!(NonZeroU32::new(width)),
            assert_some!(NonZeroU32::new(height)),
        )
    }

    fn surface(width: u32, height: u32) -> SurfaceSize {
        assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width,
            height
        }))
    }

    fn rectangle(start: (f64, f64), end: (f64, f64), size: SurfaceSize) -> SurfaceRect {
        assert_some!(SurfaceRect::from_position(
            assert_some!(SurfacePosition::new(start.0, start.1)),
            assert_some!(SurfacePosition::new(end.0, end.1)),
            size
        ))
    }

    #[rstest]
    #[case(Transform::Normal, [20.0, 10.0, 60.0, 40.0])]
    #[case(Transform::_90, [80.0, 10.0, 140.0, 30.0])]
    #[case(Transform::_180, [100.0, 40.0, 140.0, 70.0])]
    #[case(Transform::_270, [20.0, 50.0, 80.0, 70.0])]
    #[case(Transform::Flipped, [100.0, 10.0, 140.0, 40.0])]
    #[case(Transform::Flipped90, [20.0, 10.0, 80.0, 30.0])]
    #[case(Transform::Flipped180, [20.0, 40.0, 60.0, 70.0])]
    #[case(Transform::Flipped270, [80.0, 50.0, 140.0, 70.0])]
    fn maps_surface_edges_to_buffer(#[case] transform: Transform, #[case] expected: [f64; 4]) {
        let surface = surface(80, 40);
        let rectangle = rectangle((10.0, 5.0), (30.0, 20.0), surface);
        let outline = assert_ok!(Outline::new(rectangle, surface, pixels(160, 80), transform));
        let actual = [outline.left, outline.top, outline.right, outline.bottom];

        assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
    }

    #[test]
    fn draws_inward_without_changing_the_frozen_source() {
        let original = [10, 20, 30, 255];
        let image = assert_ok!(CapturedImage::new(
            pixels(8, 8),
            repeat_n(original, 64).flatten().collect()
        ));
        let surface = surface(8, 8);
        let rectangle = rectangle((1.0, 1.0), (7.0, 7.0), surface);
        let rendered = assert_ok!(selection_outline(
            &image,
            rectangle,
            surface,
            Transform::Normal
        ));

        let rows = rendered
            .rgba()
            .as_chunks::<32>()
            .0
            .iter()
            .map(|row| {
                row.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|pixel| {
                        if *pixel == BORDER_COLOR {
                            '#'
                        } else {
                            assert_eq!(*pixel, original);
                            '.'
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rows,
            [
                "........", ".######.", ".######.", ".##..##.", ".##..##.", ".######.", ".######.",
                "........",
            ]
        );
        assert!(
            image
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == original)
        );
    }
}
