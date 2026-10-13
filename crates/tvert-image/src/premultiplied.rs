use crate::{Error, region::RgbaRegion};
use image::{Rgba, Rgba32FImage};

/// Copies a region into normalized RGBA with premultiplied alpha
pub fn premultiplied_region(rgba: &[u8], region: &RgbaRegion) -> Result<Rgba32FImage, Error> {
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(region.size().rgba_len()?)?;

    for row in region.rows(rgba) {
        for &[red, green, blue, alpha] in row?.as_chunks::<4>().0 {
            let alpha = f32::from(alpha) / 255.0;
            pixels.extend([
                f32::from(red) / 255.0 * alpha,
                f32::from(green) / 255.0 * alpha,
                f32::from(blue) / 255.0 * alpha,
                alpha,
            ]);
        }
    }

    let (width, height) = region.size().into();
    Rgba32FImage::from_raw(width, height, pixels).ok_or(Error::SizeOverflow)
}

/// Converts normalized premultiplied RGBA back to straight alpha
pub fn unpremultiply(image: &mut Rgba32FImage) {
    for Rgba(channels) in image.pixels_mut() {
        let [red, green, blue, alpha] = *channels;
        *channels = if alpha > 0.0 {
            [red / alpha, green / alpha, blue / alpha, alpha]
        } else {
            [0.0; 4]
        };
    }
}
