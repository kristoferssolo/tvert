use crate::{figure::Figure, viewport::Viewport};
use iced::{
    Color, Font, Pixels, Point, Rectangle, Size, Vector,
    advanced::{
        graphics::text::Paragraph,
        text::{self, Alignment, LineHeight, Paragraph as _, Shaping, Wrapping},
    },
    alignment::Vertical,
    widget::canvas::{self, Frame, Path, Stroke, Style},
};

/// Font of text annotations, shared by the in-place editor and drawing
pub const FONT: Font = Font::DEFAULT;

/// Line height of text annotations, shared by the in-place editor and drawing
pub const LINE_HEIGHT: LineHeight = LineHeight::Relative(1.3);

/// Text drawn from its top-left corner in captured-image pixel coordinates.
///
/// Hit testing and selection use the measured text box, so clicking
/// between letters still selects the text.
#[derive(Debug)]
pub struct TextFigure {
    content: String,
    position: Point,
    /// Font size in captured-image pixels
    size: f32,
    /// Measured extent at `size`
    bounds: Size,
}

impl TextFigure {
    pub fn new(content: String, position: Point, size: f32) -> Self {
        let bounds = measure(&content, size);
        Self {
            content,
            position,
            size,
            bounds,
        }
    }
}

impl Figure for TextFigure {
    fn path(&self, viewport: Viewport) -> Path {
        let bounds = viewport.canvas_rectangle(Rectangle::new(self.position, self.bounds));
        Path::rectangle(bounds.position(), bounds.size())
    }

    fn translated(&self, offset: Vector) -> Box<dyn Figure> {
        Box::new(Self {
            content: self.content.clone(),
            position: Point::new(self.position.x + offset.x, self.position.y + offset.y),
            ..*self
        })
    }

    fn filled(&self) -> bool {
        true
    }

    /// Fills the text with the stroke color, ignoring the stroke width
    fn draw(&self, frame: &mut Frame, viewport: Viewport, stroke: Stroke<'_>) {
        let Style::Solid(color) = stroke.style else {
            return;
        };
        frame.fill_text(canvas_text(
            self.content.clone(),
            viewport.canvas_position(self.position),
            viewport.canvas_length(self.size),
            color,
        ));
    }
}

/// Returns the extent of `content` laid out at font `size`, without wrapping
pub fn measure(content: &str, size: f32) -> Size {
    Paragraph::with_text(text::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(size),
        line_height: LINE_HEIGHT,
        font: FONT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
    .min_bounds()
}

const fn canvas_text(content: String, position: Point, size: f32, color: Color) -> canvas::Text {
    canvas::Text {
        content,
        position,
        max_width: f32::INFINITY,
        color,
        size: Pixels(size),
        line_height: LINE_HEIGHT,
        font: FONT,
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hit;
    use claims::assert_some;

    fn viewport(scale: f32) -> Viewport {
        let image = Size::new(200.0, 100.0);
        assert_some!(Viewport::new(
            image,
            Rectangle::with_size(Size::new(image.width * scale, image.height * scale))
        ))
    }

    #[test]
    fn measures_each_line_at_the_line_height() {
        let one = measure("Tvert", 20.0);
        let two = measure("Tvert\nTvert", 20.0);

        assert!(one.width > 0.0);
        assert_eq!(one.width, two.width);
        assert_eq!(one.height, 26.0);
        assert_eq!(two.height, 52.0);
        assert!(measure("Tvert", 40.0).width > one.width);
    }

    #[test]
    fn the_text_box_is_hit_and_moves_with_the_text() {
        let text = TextFigure::new("Tvert".into(), Point::new(10.0, 10.0), 20.0);
        let inside = Point::new(12.0, 30.0);
        let below = Point::new(12.0, 60.0);

        assert!(hit::fill(&text.path(viewport(1.0)), inside));
        assert!(!hit::fill(&text.path(viewport(1.0)), below));

        let moved = text.translated(Vector::new(0.0, 30.0));
        assert!(hit::fill(&moved.path(viewport(1.0)), below));
        assert!(hit::fill(
            &moved.path(viewport(2.0)),
            Point::new(24.0, 120.0)
        ));
    }
}
