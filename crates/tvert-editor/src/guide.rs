use iced::{
    Color, Point, Rectangle, Size,
    theme::Palette,
    widget::canvas::{Frame, LineDash, Path, Stroke},
};

/// Space between a selected edit and its outline, in viewport pixels
const SELECTION_GAP: f32 = 4.0;

/// Draws a dashed outline that stays visible over light and dark pixels
pub fn dashed(frame: &mut Frame, path: &Path, base: Color, dash: Color) {
    let stroke = Stroke::default().with_width(1.0);

    frame.stroke(path, stroke.with_color(base));
    frame.stroke(
        path,
        Stroke {
            line_dash: LineDash {
                segments: &[6.0, 6.0],
                offset: 0,
            },
            ..stroke.with_color(dash)
        },
    );
}

/// Fills the frame outside `inner`, given in frame coordinates
pub fn fill_outside(frame: &mut Frame, inner: Rectangle, color: Color) {
    let size = frame.size();
    let Some(inner) = inner.intersection(&Rectangle::with_size(size)) else {
        frame.fill_rectangle(Point::ORIGIN, size, color);
        return;
    };
    let right = inner.x + inner.width;
    let bottom = inner.y + inner.height;
    let strips = [
        Rectangle::new(Point::ORIGIN, Size::new(size.width, inner.y)),
        Rectangle::new(
            Point::new(0.0, bottom),
            Size::new(size.width, size.height - bottom),
        ),
        Rectangle::new(Point::new(0.0, inner.y), Size::new(inner.x, inner.height)),
        Rectangle::new(
            Point::new(right, inner.y),
            Size::new(size.width - right, inner.height),
        ),
    ];
    for strip in strips {
        if strip.width > 0.0 && strip.height > 0.0 {
            frame.fill_rectangle(strip.position(), strip.size(), color);
        }
    }
}

/// Outlines the canvas `bounds` of a selected edit in theme colors
pub fn selection(frame: &mut Frame, bounds: Rectangle, palette: Palette) {
    let bounds = bounds.expand(SELECTION_GAP);
    let path = Path::rectangle(bounds.position(), bounds.size());

    dashed(frame, &path, palette.background, palette.text);
}
