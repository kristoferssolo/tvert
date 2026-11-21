use crate::{Error, pixel_effect::pixel_ranges, viewport::rectangle_between};
use iced::{Point, Rectangle, Size};
use tvert_image::CapturedImage;

/// A nonempty area of whole captured-image pixels that exports keep.
///
/// Edges lie on pixel boundaries within the image. Edits keep their
/// full-image coordinates, so a crop can be replaced by a wider one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop {
    bounds: Rectangle,
}

impl Crop {
    /// Rounds a drag between two image positions to the nearest pixel edges
    /// within an image of `image_size`.
    ///
    /// Returns `None` if the rounded drag covers no whole pixel.
    pub fn from_drag(start: Point, end: Point, image_size: Size) -> Option<Self> {
        let edge = |point: Point| {
            Point::new(
                point.x.round().clamp(0.0, image_size.width),
                point.y.round().clamp(0.0, image_size.height),
            )
        };
        rectangle_between(edge(start), edge(end)).map(|bounds| Self { bounds })
    }

    /// The kept area in captured-image coordinates
    pub const fn bounds(self) -> Rectangle {
        self.bounds
    }

    /// Copies the kept pixels out of a full-resolution export.
    ///
    /// # Errors
    ///
    /// Returns an error if the crop does not fit within `image`.
    pub fn apply(self, image: &CapturedImage) -> Result<CapturedImage, Error> {
        let (x, y) = pixel_ranges(self.bounds, image.size())?;
        Ok(image.crop(x, y)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_ok, assert_some};
    use rstest::rstest;
    use tvert_image::PixelSize;

    const IMAGE: Size = Size::new(4.0, 3.0);

    fn rectangle(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(width, height))
    }

    #[rstest]
    #[case::rounded(
        Point::new(0.6, 2.4),
        Point::new(2.4, 0.4),
        rectangle(1.0, 0.0, 1.0, 2.0)
    )]
    #[case::clipped(Point::new(-5.0, 1.0), Point::new(9.0, 7.0), rectangle(0.0, 1.0, 4.0, 2.0))]
    fn drags_snap_to_pixel_edges_within_the_image(
        #[case] start: Point,
        #[case] end: Point,
        #[case] expected: Rectangle,
    ) {
        let crop = assert_some!(Crop::from_drag(start, end, IMAGE));
        assert_eq!(crop.bounds(), expected);
    }

    #[rstest]
    #[case::within_one_pixel(Point::new(1.2, 1.2), Point::new(1.4, 2.8))]
    #[case::outside(Point::new(5.0, 0.0), Point::new(9.0, 3.0))]
    fn drags_without_a_whole_pixel_are_rejected(#[case] start: Point, #[case] end: Point) {
        assert_none!(Crop::from_drag(start, end, IMAGE));
    }

    #[test]
    fn keeps_exactly_the_covered_pixels() {
        let rgba = (0..12)
            .flat_map(|value| [value, 0, 0, 255])
            .collect::<Vec<u8>>();
        let image = assert_ok!(CapturedImage::try_from((
            assert_ok!(PixelSize::try_from((4, 3))),
            rgba.as_slice()
        )));
        let crop = assert_some!(Crop::from_drag(
            Point::new(1.0, 1.0),
            Point::new(3.0, 3.0),
            IMAGE
        ));

        let cropped = assert_ok!(crop.apply(&image));

        assert_eq!(cropped.size(), assert_ok!(PixelSize::try_from((2, 2))));
        let reds = cropped
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[red, _, _, _]| red)
            .collect::<Vec<_>>();
        assert_eq!(reds, [5, 6, 9, 10]);
    }
}
