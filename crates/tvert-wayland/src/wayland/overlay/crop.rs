use crate::{
    Error,
    wayland::{
        output::buffer_to_surface,
        overlay::{SurfaceRect, geometry::PixelRect, size::SurfaceSize},
    },
};
use tvert_image::CapturedImage;
use wayland_client::protocol::wl_output::Transform;

/// Copies a surface selection and orients its pixels as displayed.
///
/// The frozen source is preserved. Returns `None` if the mapped pixel ranges
/// are empty. The crop is taken before its pixels are rotated or reflected.
///
/// # Errors
///
/// Returns [`Error::UnsupportedOutputTransform`] for an unsupported transform,
/// [`Error::InvalidShmLayout`] if pixel bounds cannot be represented, or
/// [`Error::Image`] if cropping or transforming the image fails.
pub fn selection(
    image: &CapturedImage,
    rectangle: SurfaceRect,
    surface: SurfaceSize,
    transform: Transform,
) -> Result<Option<CapturedImage>, Error> {
    let Some(bounds) = PixelRect::from_surface(rectangle, surface, image.size(), transform)? else {
        return Ok(None);
    };

    let image = image
        .crop(bounds.x().clone(), bounds.y().clone())?
        .transformed(buffer_to_surface(transform)?)?;

    Ok(Some(image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_ok, assert_some};
    use rstest::rstest;
    use std::num::NonZeroU32;
    use tvert_image::PixelSize;

    fn pixels(width: u32, height: u32) -> PixelSize {
        PixelSize::new(
            assert_some!(NonZeroU32::new(width)),
            assert_some!(NonZeroU32::new(height)),
        )
    }

    // Frozen buffer:
    //  0  1  2  3
    //  4  5  6  7
    //  8  9 10 11
    //
    // Select the first two pixels of the displayed top row.
    // Surface dimensions use twice the corresponding pixel dimensions.
    #[rstest]
    #[case(Transform::Normal, 8, 6, [0, 1])]
    #[case(Transform::_90, 6, 8, [8, 4])]
    #[case(Transform::_180, 8, 6, [11, 10])]
    #[case(Transform::_270, 6, 8, [3, 7])]
    #[case(Transform::Flipped, 8, 6, [3, 2])]
    #[case(Transform::Flipped90, 6, 8, [0, 4])]
    #[case(Transform::Flipped180, 8, 6, [8, 9])]
    #[case(Transform::Flipped270, 6, 8, [11, 7])]
    fn crops_in_displayed_orientation(
        #[case] transform: Transform,
        #[case] width: u32,
        #[case] height: u32,
        #[case] expected: [u8; 2],
    ) {
        let original = (0..12)
            .flat_map(|value| [value, 20, 30, value])
            .collect::<Vec<_>>();
        let image = assert_ok!(CapturedImage::new(pixels(4, 3), original.clone()));
        let surface = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width,
            height,
        }));
        let rectangle = assert_some!(SurfaceRect::from_position(
            assert_some!(SurfacePosition::new(0.0, 0.0)),
            assert_some!(SurfacePosition::new(4.0, 2.0)),
            surface,
        ));

        let result = assert_some!(assert_ok!(selection(&image, rectangle, surface, transform)));
        let expected = expected
            .into_iter()
            .flat_map(|value| [value, 20, 30, value])
            .collect::<Vec<_>>();

        assert_eq!(result.size(), pixels(2, 1));
        assert_eq!(result.rgba(), expected);
        assert_eq!(image.rgba(), original);
    }
}
