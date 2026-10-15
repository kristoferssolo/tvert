use crate::{
    Error,
    wayland::overlay::{SurfaceRect, geometry::buffer_edges, size::SurfaceSize},
};
use tvert_image::{CapturedImage, ImageTransform, PixelSize};

const BORDER_LIGHT: [u8; 4] = [255, 255, 255, 255];
const BORDER_DARK: [u8; 4] = [0, 0, 0, 255];
const BORDER_WIDTH: f64 = 2.0;
const DASH_LENGTH: f64 = 6.0;

/// Dims the frozen image outside the selection and draws its dashed outline.
///
/// With no rectangle, dims the whole image. Coordinates and border widths use
/// surface units, with coverage determined by buffer pixel centers. The source,
/// buffer dimensions, orientation, and non-border alpha values are preserved.
///
/// # Errors
///
/// Returns [`Error::InvalidShmLayout`] if row addressing cannot be represented,
/// [`Error::ImageAllocation`] if allocating the copy fails, or [`Error::Image`]
/// if constructing the resulting image fails.
pub fn selection_preview(
    image: &CapturedImage,
    rectangle: Option<SurfaceRect>,
    surface_size: SurfaceSize,
    transform: ImageTransform,
) -> Result<CapturedImage, Error> {
    let outline =
        rectangle.map(|rectangle| Outline::new(rectangle, surface_size, image.size(), transform));
    let row_bytes = image
        .size()
        .row_bytes()
        .map_err(|_| Error::InvalidShmLayout)?;

    let mut rgba = Vec::new();
    rgba.try_reserve_exact(image.rgba().len())?;
    rgba.extend_from_slice(image.rgba());

    for (y, row) in rgba.chunks_exact_mut(row_bytes).enumerate() {
        let y = f64::from(u32::try_from(y).map_err(|_| Error::InvalidShmLayout)?) + 0.5;

        for (x, pixel) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let x = f64::from(u32::try_from(x).map_err(|_| Error::InvalidShmLayout)?) + 0.5;

            if let Some(outline) = outline.as_ref()
                && outline.covers(x, y)
            {
                if let Some(color) = outline.color(x, y) {
                    *pixel = color;
                }
                continue;
            }

            let [red, green, blue, _] = pixel;
            *red >>= 1;
            *green >>= 1;
            *blue >>= 1;
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
    scale_x: f64,
    scale_y: f64,
}

impl Outline {
    fn new(
        rectangle: SurfaceRect,
        surface: SurfaceSize,
        pixels: PixelSize,
        transform: ImageTransform,
    ) -> Self {
        let [left, top, right, bottom] = buffer_edges(rectangle, surface, pixels, transform);
        let surface_width = f64::from(surface.width());
        let surface_height = f64::from(surface.height());

        let (axis_width, axis_height) = if transform.swaps_axes() {
            (surface_height, surface_width)
        } else {
            (surface_width, surface_height)
        };

        let scale_x = f64::from(pixels.width().get()) / axis_width;
        let scale_y = f64::from(pixels.height().get()) / axis_height;

        Self {
            left,
            top,
            right,
            bottom,
            stroke_x: BORDER_WIDTH * scale_x,
            stroke_y: BORDER_WIDTH * scale_y,
            scale_x,
            scale_y,
        }
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        if !self.covers(x, y) {
            return false;
        }

        x < self.left + self.stroke_x
            || x >= self.right - self.stroke_x
            || y < self.top + self.stroke_y
            || y >= self.bottom - self.stroke_y
    }

    fn color(&self, x: f64, y: f64) -> Option<[u8; 4]> {
        if !self.contains(x, y) {
            return None;
        }

        let width = (self.right - self.left) / self.scale_x;
        let height = (self.bottom - self.top) / self.scale_y;
        let distance = if y < self.top + self.stroke_y {
            (x - self.left) / self.scale_x
        } else if x >= self.right - self.stroke_x {
            width + (y - self.top) / self.scale_y
        } else if y >= self.bottom - self.stroke_y {
            width + height + (self.right - x) / self.scale_x
        } else {
            2.0f64.mul_add(width, height) + (self.bottom - y) / self.scale_y
        };

        Some(if distance.rem_euclid(2.0 * DASH_LENGTH) < DASH_LENGTH {
            BORDER_LIGHT
        } else {
            BORDER_DARK
        })
    }

    fn covers(&self, x: f64, y: f64) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_ok, assert_some};
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

    #[test]
    fn dims_outside_the_selection_without_changing_the_frozen_source() {
        let original = [10, 20, 30, 255];
        let image = assert_ok!(CapturedImage::new(
            pixels(8, 8),
            repeat_n(original, 64).flatten().collect()
        ));
        let surface = surface(8, 8);
        let rectangle = rectangle((1.0, 1.0), (7.0, 7.0), surface);
        let rendered = assert_ok!(selection_preview(
            &image,
            Some(rectangle),
            surface,
            ImageTransform::Normal
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
                        if matches!(*pixel, BORDER_LIGHT | BORDER_DARK) {
                            '#'
                        } else if *pixel == original {
                            '.'
                        } else {
                            assert_eq!(*pixel, [5, 10, 15, 255]);
                            'd'
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rows,
            [
                "dddddddd", "d######d", "d######d", "d##..##d", "d##..##d", "d######d", "d######d",
                "dddddddd",
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

    #[test]
    fn draws_a_fractional_selection_containing_one_pixel() {
        let image = assert_ok!(CapturedImage::new(
            pixels(8, 8),
            repeat_n([10, 20, 30, 255], 64).flatten().collect()
        ));
        let surface = surface(8, 8);
        let rectangle = rectangle((1.1, 1.1), (2.2, 2.2), surface);
        let rendered = assert_ok!(selection_preview(
            &image,
            Some(rectangle),
            surface,
            ImageTransform::Normal
        ));
        let pixels = rendered.rgba().as_chunks::<4>().0;

        assert_eq!(assert_some!(pixels.get(9)), &BORDER_LIGHT);
        assert_eq!(
            pixels
                .iter()
                .filter(|pixel| matches!(**pixel, BORDER_LIGHT | BORDER_DARK))
                .count(),
            1
        );
    }

    #[test]
    fn dims_without_a_selection_and_preserves_alpha() {
        let original = vec![255, 101, 1, 128, 17, 33, 65, 0];
        let image = assert_ok!(CapturedImage::new(pixels(2, 1), original.clone()));

        let rendered = assert_ok!(selection_preview(
            &image,
            None,
            surface(2, 1),
            ImageTransform::Normal
        ));

        assert_eq!(rendered.rgba(), [127, 50, 0, 128, 8, 16, 32, 0]);
        assert_eq!(image.rgba(), original);
    }
}
