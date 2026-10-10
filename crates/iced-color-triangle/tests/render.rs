use claims::assert_some;
use iced_color_triangle::{Selection, color_triangle};
use iced_core::{
    Color, Element, Font, Layout, Pixels, Rectangle, Size, Theme,
    layout::Limits,
    mouse::Cursor,
    renderer::{Renderer as _, Style},
    widget::Tree,
};
use iced_tiny_skia::{
    Layer, Renderer,
    graphics::{Viewport, damage},
};
use tiny_skia::{Mask, Pixmap};

const BOUNDS: Rectangle = Rectangle {
    x: 0.0,
    y: 0.0,
    width: 240.0,
    height: 240.0,
};
const BACKGROUND: Color = Color::from_rgb8(40, 42, 44);

fn paint(renderer: &mut Renderer, tree: &mut Tree, selection: Selection) {
    let mut element: Element<'_, Selection, Theme, Renderer> =
        color_triangle(selection, std::convert::identity);
    tree.diff(element.as_widget());
    let node =
        element
            .as_widget_mut()
            .layout(tree, renderer, &Limits::new(Size::ZERO, BOUNDS.size()));
    renderer.reset(BOUNDS);
    element.as_widget().draw(
        tree,
        renderer,
        &Theme::Dark,
        &Style {
            text_color: Color::WHITE,
        },
        Layout::new(&node),
        Cursor::Unavailable,
        &BOUNDS,
    );
}

#[test]
fn successive_redraws_match_a_fresh_frame() {
    for physical_side in [240, 300, 480] {
        let scale = match physical_side {
            300 => 1.25,
            480 => 2.0,
            _ => 1.0,
        };
        let viewport = Viewport::with_physical_size(Size::new(physical_side, physical_side), scale);
        let mut incremental = assert_some!(Pixmap::new(physical_side, physical_side));
        let mut expected = assert_some!(Pixmap::new(physical_side, physical_side));
        let mut mask = assert_some!(Mask::new(physical_side, physical_side));
        let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
        let initial = Selection::from(Color::from_rgb8(20, 30, 180));
        let element: Element<'_, Selection, Theme, Renderer> =
            color_triangle(initial, std::convert::identity);
        let mut tree = Tree::new(element.as_widget());
        let mut previous: Option<Vec<Layer>> = None;
        for step in 0..4u8 {
            let selection = initial.with_lightness(f32::from(step).mul_add(0.06, 0.12));
            paint(&mut renderer, &mut tree, selection);
            let regions = previous.as_ref().map_or_else(
                || vec![BOUNDS],
                |previous| {
                    damage::group(
                        damage::diff(
                            previous,
                            renderer.layers(),
                            |layer| vec![layer.bounds],
                            Layer::damage,
                        ),
                        BOUNDS,
                    )
                },
            );
            previous = Some(renderer.layers().to_vec());
            renderer.draw(
                &mut incremental.as_mut(),
                &mut mask,
                &viewport,
                &regions,
                BACKGROUND,
            );
            renderer.draw(
                &mut expected.as_mut(),
                &mut mask,
                &viewport,
                &[BOUNDS],
                BACKGROUND,
            );
            let different = incremental
                .data()
                .iter()
                .zip(expected.data())
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(
                different, 0,
                "scale {scale}, step {step}: incremental damage left stale pixels"
            );
        }
    }
}

#[test]
fn rotated_triangle_has_no_background_gaps() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let initial = Selection::from(Color::from_rgb8(20, 30, 180));
    let element: Element<'_, Selection, Theme, Renderer> =
        color_triangle(initial, std::convert::identity);
    let mut tree = Tree::new(element.as_widget());
    for hue in [0.0, 60.0, 120.0, 180.0, 240.0, 300.0] {
        for side in [240_u32, 300, 480] {
            let scale = match side {
                300 => 1.25,
                480 => 2.0,
                _ => 1.0,
            };
            let selection = initial
                .with_hue(hue)
                .with_saturation(1.0)
                .with_lightness(0.5);
            paint(&mut renderer, &mut tree, selection);
            let mut pixmap = assert_some!(Pixmap::new(side, side));
            let mut mask = assert_some!(Mask::new(side, side));
            let viewport = Viewport::with_physical_size(Size::new(side, side), scale);
            renderer.draw(
                &mut pixmap.as_mut(),
                &mut mask,
                &viewport,
                &[BOUNDS],
                BACKGROUND,
            );
            let center = side.midpoint(0);
            for y in center.saturating_sub(8)..=center.saturating_add(8) {
                for x in center.saturating_sub(8)..=center.saturating_add(8) {
                    let pixel = assert_some!(pixmap.pixel(x, y));
                    assert!(
                        pixel.red().min(pixel.green()).min(pixel.blue()) >= 50,
                        "background gap at ({x}, {y}), hue {hue}, scale {scale}"
                    );
                }
            }
        }
    }
}
