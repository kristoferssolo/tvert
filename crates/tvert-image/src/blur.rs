use crate::{
    Error, PixelSize,
    premultiplied::{premultiplied_region, unpremultiply},
    region::RgbaRegion,
};
use image::{RgbaImage, buffer::ConvertBuffer, imageops};
use std::ops::Range;

pub fn blur(
    rgba: &mut [u8],
    size: PixelSize,
    x: Range<usize>,
    y: Range<usize>,
    sigma: f32,
) -> Result<(), Error> {
    if !sigma.is_normal() || sigma <= 0.0 {
        return Err(Error::InvalidBlurSigma);
    }

    let region = RgbaRegion::new(size, x, y)?.ok_or(Error::InvalidBlurBounds)?;
    let source = premultiplied_region(rgba, &region)?;
    let mut output = imageops::blur(&source, sigma);
    unpremultiply(&mut output);

    let output: RgbaImage = output.convert();
    let row_bytes = region.size().row_byes()?;

    for (row, blurred) in region
        .rows_mut(rgba)
        .zip(output.as_raw().chunks_exact(row_bytes))
    {
        row?.copy_from_slice(blurred);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapturedImage;
    use claims::{assert_err, assert_ok, assert_some};
    use rstest::rstest;
    use std::assert_matches;

    fn image(width: u32, height: u32, rgba: Vec<u8>) -> CapturedImage {
        let size = assert_ok!(PixelSize::try_from((width, height)));
        assert_ok!(CapturedImage::new(size, rgba))
    }

    #[test]
    fn blurs_in_two_dimensions_without_sampling_or_changing_surrounding_pixels() {
        let mut images = [0, 200].map(|outside| {
            let mut pixels = [outside, 0, 0, 255].repeat(25);
            for row in 1..4 {
                for column in 1..4 {
                    let index = row * 5 + column;
                    let pixel = assert_some!(pixels.as_chunks_mut::<4>().0.get_mut(index));
                    *pixel = [0, 0, 0, 255];
                }
            }

            *assert_some!(pixels.as_chunks_mut::<4>().0.get_mut(12)) = [255, 0, 0, 255];
            image(5, 5, pixels)
        });

        for image in &mut images {
            let original = image.rgba().to_vec();
            assert_ok!(image.blur(1..4, 1..4, 1.0));

            for (index, (pixel, source)) in image
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .zip(original.as_chunks::<4>().0)
                .enumerate()
            {
                let inside = (1..4).contains(&(index / 5)) && (1..4).contains(&(index % 5));
                if !inside {
                    assert_eq!(pixel, source);
                }
            }

            let cropped = assert_ok!(image.crop(1..4, 1..4));
            let reds = cropped
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&[red, _, _, _]| red)
                .collect::<Vec<_>>();

            let center = *assert_some!(reds.get(4));
            let horizontal = *assert_some!(reds.get(3));
            let vertical = *assert_some!(reds.get(1));

            assert!(center < 255 && center > horizontal);
            assert!(horizontal > 0 && vertical > 0);
            assert_eq!(horizontal, vertical);
        }

        let [dark, light] = images;
        assert_eq!(
            assert_ok!(dark.crop(1..4, 1..4)).rgba(),
            assert_ok!(light.crop(1..4, 1..4)).rgba()
        );
    }

    #[test]
    fn sigma_changes_how_far_the_blur_spreads() {
        let mut pixels = [0, 0, 0, 255].repeat(49);
        *assert_some!(pixels.as_chunks_mut::<4>().0.get_mut(24)) = [255, 0, 0, 255];

        let mut narrow = image(7, 7, pixels.clone());
        let mut wide = image(7, 7, pixels);

        assert_ok!(narrow.blur(0..7, 0..7, 0.5));
        assert_ok!(wide.blur(0..7, 0..7, 2.0));

        let [narrow_red, _, _, _] = *assert_some!(narrow.rgba().as_chunks::<4>().0.get(24));
        let [wide_red, _, _, _] = *assert_some!(wide.rgba().as_chunks::<4>().0.get(24));
        assert!(narrow_red > wide_red);
    }

    #[rstest]
    #[case(255)]
    #[case(0)]
    fn transparent_pixels_do_not_tint_the_blur(#[case] alpha: u8) {
        let mut image = image(2, 1, vec![255, 0, 0, alpha, 0, 0, 255, 0]);
        assert_ok!(image.blur(0..2, 0..1, 1.0));

        for &[red, green, blue, blurred_alpha] in image.rgba().as_chunks::<4>().0 {
            if alpha == 0 {
                assert_eq!([red, green, blue, blurred_alpha], [0; 4]);
            } else {
                assert_eq!([red, green, blue], [255, 0, 0]);
                assert!(blurred_alpha > 0 && blurred_alpha < 255);
            }
        }
    }

    #[rstest]
    #[case(0.0)]
    #[case(-1.0)]
    #[case(f32::NAN)]
    #[case(f32::INFINITY)]
    #[case(f32::MIN_POSITIVE / 2.0)]
    fn rejects_invalid_sigma_without_changing_pixels(#[case] sigma: f32) {
        let original = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let mut image = image(2, 1, original.clone());

        let error = assert_err!(image.blur(0..2, 0..1, sigma));
        assert_matches!(error, Error::InvalidBlurSigma);
        assert_eq!(image.rgba(), original);
    }

    #[test]
    fn rejects_invalid_bounds_without_changing_pixels() {
        let original = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let mut image = image(2, 1, original.clone());

        let error = assert_err!(image.blur(0..3, 0..1, 1.0));
        assert_matches!(error, Error::InvalidBlurBounds);
        assert_eq!(image.rgba(), original);
    }
}
