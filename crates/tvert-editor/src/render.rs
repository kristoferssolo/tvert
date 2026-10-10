use crate::{Error, annotation::Annotation, edit::Edit, viewport::Viewport};
use iced::{
    Color, Font, Pixels, Renderer, Size,
    advanced::{
        graphics::geometry::{Frame, Renderer as _},
        renderer::Headless as _,
    },
    widget::image::Handle,
};
use tvert_image::{CapturedImage, Error as ImageError, PixelSize};

/// Renders drawings over decoded image pixels at their original size.
///
/// Pixels outside the drawings retain their original straight RGBA values.
///
/// # Errors
///
/// Returns an error for an unsupported preview handle, invalid image storage,
/// an invalid viewport, or an allocation or compositing failure.
pub fn render_image(preview: &Handle, annotations: &[Annotation]) -> Result<CapturedImage, Error> {
    draw_annotations(image_from_handle(preview)?, annotations.iter())
}

/// Replays drawings and pixel effects in their original history order.
///
/// # Errors
///
/// Returns an error if preparing, rendering, or applying an edit fails.
pub fn render_edits(preview: &Handle, edits: &[Edit]) -> Result<CapturedImage, Error> {
    let mut image = image_from_handle(preview)?;
    let mut remaining = edits;

    while let Some((edit, rest)) = remaining.split_first() {
        match edit {
            Edit::Pixelation(effect) => {
                effect.apply(&mut image)?;
                remaining = rest;
            }
            Edit::Annotation(_) => {
                let count = remaining
                    .iter()
                    .take_while(|edit| matches!(edit, Edit::Annotation(_)))
                    .count();
                let (annotations, rest) = remaining.split_at(count);
                image = draw_annotations(image, annotations.iter().filter_map(Edit::annotation))?;
                remaining = rest;
            }
        }
    }

    Ok(image)
}

fn image_from_handle(preview: &Handle) -> Result<CapturedImage, Error> {
    let Handle::Rgba {
        width,
        height,
        pixels,
        ..
    } = preview
    else {
        return Err(Error::InvalidPreview);
    };

    let size = PixelSize::try_from((*width, *height))?;
    let expected = size.rgba_len()?;

    if pixels.len() != expected {
        return Err(ImageError::InvalidPixelLength {
            expected,
            actual: pixels.len(),
        }
        .into());
    }

    let mut rgba = Vec::new();
    rgba.try_reserve_exact(expected).map_err(ImageError::from)?;
    rgba.extend_from_slice(pixels);

    Ok(CapturedImage::new(size, rgba)?)
}

fn draw_annotations<'a>(
    image: CapturedImage,
    annotations: impl Iterator<Item = &'a Annotation>,
) -> Result<CapturedImage, Error> {
    let mut annotations = annotations.peekable();
    if annotations.peek().is_none() {
        return Ok(image);
    }

    let size = image.size();
    let (width, height) = size.into();
    let image_size = Size::from((width, height));
    let viewport = Viewport::new(image_size, image_size).ok_or(Error::InvalidViewport)?;
    let mut rendered = Renderer::new(Font::default(), Pixels(16.0));
    let mut frame = Frame::new(&rendered, image_size);

    for annotation in annotations {
        annotation.draw(&mut frame, viewport);
    }

    rendered.draw_geometry(frame.into_geometry());

    let overlay = rendered.screenshot(Size::new(width, height), 1.0, Color::TRANSPARENT);
    let expected = size.rgba_len()?;
    if overlay.len() != expected {
        return Err(ImageError::InvalidPixelLength {
            expected,
            actual: overlay.len(),
        }
        .into());
    }

    let mut rgba = image.into_rgba();

    for (background, foreground) in rgba
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(overlay.as_chunks::<4>().0)
    {
        composite(background, *foreground)?;
    }

    Ok(CapturedImage::new(size, rgba)?)
}

fn composite(background: &mut [u8; 4], foreground: [u8; 4]) -> Result<(), ImageError> {
    let [red, green, blue, alpha] = foreground;

    if alpha == 0 {
        return Ok(());
    }

    if alpha == u8::MAX {
        *background = foreground;
        return Ok(());
    }

    let [base_red, base_green, base_blue, base_alpha] = *background;
    let inverse = u32::from(u8::MAX.saturating_sub(alpha));

    let base_weight = u32::from(base_alpha)
        .checked_mul(inverse)
        .ok_or(ImageError::SizeOverflow)?;

    let alpha_weight = u32::from(alpha)
        .checked_mul(255)
        .and_then(|value| value.checked_add(base_weight))
        .ok_or(ImageError::SizeOverflow)?;

    // Iced's tiny-skia screenshot has premultiplied channels. Convert directly
    // to straight RGBA while combining it with the original straight pixels.
    let channel = |foreground: u8, background: u8| -> Result<u8, ImageError> {
        let base = u32::from(background)
            .checked_mul(base_weight)
            .ok_or(ImageError::SizeOverflow)?;

        let value = u32::from(foreground)
            .checked_mul(65_025)
            .and_then(|value| value.checked_add(base))
            .and_then(|value| value.checked_add(alpha_weight / 2))
            .and_then(|value| value.checked_div(alpha_weight))
            .ok_or(ImageError::SizeOverflow)?;

        u8::try_from(value).map_err(|_| ImageError::SizeOverflow)
    };

    *background = [
        channel(red, base_red)?,
        channel(green, base_green)?,
        channel(blue, base_blue)?,
        u8::try_from(
            alpha_weight
                .checked_add(127)
                .ok_or(ImageError::SizeOverflow)?
                / 255,
        )
        .map_err(|_| ImageError::SizeOverflow)?,
    ];

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        annotation::AnnotationDraft, block_size::BlockSize, stroke_width::StrokeWidth, tool::Tool,
    };
    use claims::{assert_ok, assert_some};
    use iced::Point;

    fn line(color: Color, width: StrokeWidth, start: Point, end: Point) -> Annotation {
        assert_ok!(Annotation::try_from(AnnotationDraft {
            tool: Tool::Line,
            stroke_width: width,
            stroke_color: color,
            block_size: BlockSize::default(),
            points: vec![start, end,]
        }))
    }

    fn pixel(image: &CapturedImage, x: usize, y: usize) -> [u8; 4] {
        let width = assert_ok!(usize::try_from(image.size().width().get()));
        let position = assert_some!(y.checked_mul(width).and_then(|row| row.checked_add(x)));
        *assert_some!(image.rgba().as_chunks::<4>().0.get(position))
    }

    #[test]
    fn preserves_all_source_bytes_without_annotations() {
        let pixels = [23, 47, 91, 0, 11, 77, 123, 128];
        let preview = Handle::from_rgba(2, 1, pixels.to_vec());

        let image = assert_ok!(render_image(&preview, &[]));

        assert_eq!(image.rgba(), pixels);
    }

    #[test]
    fn renders_retained_strokes_in_order_at_image_resolution() {
        let preview = Handle::from_rgba(32, 32, [20, 40, 60, 255].repeat(1024));
        let annotations = [
            line(
                Color::from_rgb8(255, 0, 0),
                StrokeWidth::Thin,
                Point::new(4.0, 16.0),
                Point::new(28.0, 16.0),
            ),
            line(
                Color::from_rgb8(0, 0, 255),
                StrokeWidth::Thick,
                Point::new(16.0, 4.0),
                Point::new(16.0, 28.0),
            ),
        ];

        let image = assert_ok!(render_image(&preview, &annotations));

        assert_eq!(image.size(), assert_ok!(PixelSize::try_from((32, 32))));
        assert_eq!(pixel(&image, 0, 0), [20, 40, 60, 255]);
        assert_eq!(pixel(&image, 8, 16), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 16, 16), [0, 0, 255, 255]);
        assert_eq!(pixel(&image, 19, 16), [0, 0, 255, 255]);
        assert_eq!(pixel(&image, 20, 16), [255, 0, 0, 255]);
    }

    #[test]
    fn composites_transparent_strokes_without_darkening_the_result() {
        let pixels = [0, 0, 255, 128].repeat(1024);
        let preview = Handle::from_rgba(32, 32, pixels);
        let annotations = [line(
            Color::from_rgba8(255, 0, 0, 0.5),
            StrokeWidth::Medium,
            Point::new(4.0, 16.0),
            Point::new(28.0, 16.0),
        )];

        let image = assert_ok!(render_image(&preview, &annotations));

        assert_eq!(pixel(&image, 0, 0), [0, 0, 255, 128]);
        assert_eq!(pixel(&image, 8, 16), [170, 0, 85, 192]);
    }

    #[test]
    fn exports_a_pencil_loop_using_its_intermediate_points() {
        let preview = Handle::from_rgba(32, 32, [20, 40, 60, 255].repeat(1024));
        let annotation = assert_ok!(Annotation::try_from(AnnotationDraft {
            tool: Tool::Pencil,
            stroke_width: StrokeWidth::Medium,
            stroke_color: Color::from_rgb8(255, 0, 0),
            block_size: BlockSize::default(),
            points: vec![
                Point::new(4.0, 4.0),
                Point::new(24.0, 4.0),
                Point::new(24.0, 24.0),
                Point::new(4.0, 4.0),
            ],
        }));

        let image = assert_ok!(render_image(&preview, &[annotation]));

        assert_eq!(pixel(&image, 12, 4), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 24, 12), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 0, 0), [20, 40, 60, 255]);
        assert_eq!(pixel(&image, 24, 4), [20, 40, 60, 255]);
    }

    #[test]
    fn fills_a_highlight_without_hiding_the_original_pixels() {
        let preview = Handle::from_rgba(32, 32, [255, 255, 255, 255].repeat(1024));
        let highlight = assert_ok!(Annotation::try_from(AnnotationDraft {
            tool: Tool::Highlighter,
            stroke_width: StrokeWidth::Medium,
            stroke_color: Color::from_rgb8(255, 255, 0),
            block_size: BlockSize::default(),
            points: vec![Point::new(24.0, 24.0), Point::new(4.0, 4.0)],
        }));

        let image = assert_ok!(render_image(&preview, &[highlight]));

        assert_eq!(pixel(&image, 12, 12), [255, 255, 178, 255]);
        assert_eq!(pixel(&image, 0, 0), [255, 255, 255, 255]);
    }

    #[test]
    fn pixelates_earlier_drawings_and_keeps_later_drawings_on_top() {
        let preview = Handle::from_rgba(32, 16, [0, 0, 0, 255].repeat(512));
        let mut edits = vec![
            Edit::Annotation(line(
                Color::from_rgb8(255, 0, 0),
                StrokeWidth::Medium,
                Point::new(0.0, 8.0),
                Point::new(32.0, 8.0),
            )),
            assert_ok!(Edit::try_from(AnnotationDraft {
                tool: Tool::Pixelate,
                stroke_width: StrokeWidth::Medium,
                stroke_color: Color::BLACK,
                block_size: BlockSize::default(),
                points: vec![Point::new(16.0, 16.0), Point::ORIGIN],
            })),
        ];

        let image = assert_ok!(render_edits(&preview, &edits));
        assert_eq!(pixel(&image, 8, 8), [63, 0, 0, 255]);
        assert_eq!(pixel(&image, 24, 8), [255, 0, 0, 255]);

        edits.push(Edit::Annotation(line(
            Color::from_rgb8(0, 0, 255),
            StrokeWidth::Thin,
            Point::new(8.0, 0.0),
            Point::new(8.0, 16.0),
        )));
        let image = assert_ok!(render_edits(&preview, &edits));
        assert_eq!(pixel(&image, 8, 8), [0, 0, 255, 255]);
    }
}
