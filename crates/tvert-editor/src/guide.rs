use iced::{
    Color, Rectangle,
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

/// Outlines the canvas `bounds` of a selected edit in theme colors
pub fn selection(frame: &mut Frame, bounds: Rectangle, palette: Palette) {
    let bounds = bounds.expand(SELECTION_GAP);
    let path = Path::rectangle(bounds.position(), bounds.size());

    dashed(frame, &path, palette.background, palette.text);
}
