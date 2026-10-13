use iced::{Point, Rectangle, Size, Vector};

/// Largest zoom in viewport pixels per image pixel, unless fitting needs more
const MAX_SCALE: f32 = 32.0;

/// How the image is placed in the canvas.
///
/// Session state: it never enters configuration, history, or retained geometry.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Navigation {
    /// Shows the whole image, refitting whenever the canvas resizes
    #[default]
    Fit,
    /// Shows `scale` viewport pixels per image pixel, with the image point
    /// `center` at the canvas center
    Manual { scale: f32, center: Point },
}

/// Maps between captured-image pixels and canvas coordinates for one
/// navigation and canvas size
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// Displayed image in canvas coordinates, where the renderer places its pixels
    pub bounds: Rectangle,
    image_size: Size,
    canvas: Rectangle,
    /// Scale that fits the whole image
    fit: f32,
    scale: f32,
    center: Point,
}

impl Viewport {
    /// Fits the whole image in `canvas`
    pub fn new(image_size: Size, canvas: Rectangle) -> Option<Self> {
        Self::navigated(image_size, canvas, Navigation::Fit)
    }

    /// Places the image in `canvas`, given in window coordinates, as `navigation` requests.
    ///
    /// Returns `None` for empty or non-finite sizes and placements.
    pub fn navigated(image_size: Size, canvas: Rectangle, navigation: Navigation) -> Option<Self> {
        if ![
            image_size.width,
            image_size.height,
            canvas.width,
            canvas.height,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
        {
            return None;
        }

        let fit = (canvas.width / image_size.width).min(canvas.height / image_size.height);
        let (scale, center) = match navigation {
            Navigation::Fit => (
                fit,
                Point::new(image_size.width / 2.0, image_size.height / 2.0),
            ),
            Navigation::Manual { scale, center } => (scale, center),
        };
        let placement = place(image_size, canvas.size(), scale, center);
        let usable = [fit, scale, placement.width, placement.height]
            .into_iter()
            .all(is_positive)
            && placement.x.is_finite()
            && placement.y.is_finite();

        if !usable {
            return None;
        }

        Some(Self {
            bounds: Rectangle::new(
                Point::new(
                    snap(canvas.x + placement.x, placement.width / image_size.width) - canvas.x,
                    snap(canvas.y + placement.y, placement.height / image_size.height) - canvas.y,
                ),
                placement.size(),
            ),
            image_size,
            canvas,
            fit,
            scale,
            center,
        })
    }

    /// Returns the image rectangle to request from the renderer, in canvas coordinates.
    ///
    /// Tiny-skia truncates the origin toward zero to whole image pixels, as
    /// [`Self::bounds`] mirrors, but repaints only around the requested rectangle.
    /// Requesting the drawn origin, nudged inside its pixel against float error,
    /// keeps that repaint over the old image, so moving the view leaves no stale strips.
    pub fn placement(self) -> Rectangle {
        // Under half an image pixel, so truncation still lands on `bounds`, and
        // half a logical pixel, within the renderer's one-pixel damage margin
        let nudge = (self.scale / 2.0).min(0.5);
        let inside = |origin: f32, canvas: f32| origin + nudge.copysign(canvas + origin);
        Rectangle::new(
            Point::new(
                inside(self.bounds.x, self.canvas.x),
                inside(self.bounds.y, self.canvas.y),
            ),
            self.bounds.size(),
        )
    }

    pub fn zoom(self, anchor: Point, factor: f32) -> Option<Navigation> {
        let scale = (self.scale * factor).clamp(self.fit.min(1.0), self.fit.max(MAX_SCALE));
        if scale.total_cmp(&self.scale).is_eq() {
            return None;
        }

        // `anchor` shows the image point `center + (anchor - canvas center) / scale`,
        // so moving `center` by the change in `1 / scale` keeps that point fixed
        let shift = self.scale.recip() - scale.recip();
        self.manual(
            scale,
            Point::new(
                (anchor.x - self.canvas.width / 2.0).mul_add(shift, self.center.x),
                (anchor.y - self.canvas.height / 2.0).mul_add(shift, self.center.y),
            ),
        )
    }

    /// Moves the view so the image follows a canvas `offset`
    pub fn pan(self, offset: Vector) -> Option<Navigation> {
        self.manual(
            self.scale,
            Point::new(
                self.center.x - offset.x / self.scale,
                self.center.y - offset.y / self.scale,
            ),
        )
    }

    pub fn image_position(self, point: Point) -> Option<Point> {
        if !self.bounds.contains(point) {
            return None;
        }
        self.clamped_image_position(point)
    }

    pub fn clamped_image_position(self, point: Point) -> Option<Point> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }

        Some(Point::new(
            ((point.x - self.bounds.x) / self.bounds.width).clamp(0.0, 1.0) * self.image_size.width,
            ((point.y - self.bounds.y) / self.bounds.height).clamp(0.0, 1.0)
                * self.image_size.height,
        ))
    }

    /// Maps an image-pixel position into canvas coordinates
    pub fn canvas_position(self, point: Point) -> Point {
        Point::new(
            (point.x / self.image_size.width).mul_add(self.bounds.width, self.bounds.x),
            (point.y / self.image_size.height).mul_add(self.bounds.height, self.bounds.y),
        )
    }

    /// Maps an image-pixel rectangle into canvas coordinates
    pub fn canvas_rectangle(self, rectangle: Rectangle) -> Rectangle {
        Rectangle::new(
            self.canvas_position(rectangle.position()),
            Size::new(
                rectangle.width / self.image_size.width * self.bounds.width,
                rectangle.height / self.image_size.height * self.bounds.height,
            ),
        )
    }

    /// Maps an image-pixel length into canvas units
    pub fn canvas_length(&self, pixels: f32) -> f32 {
        pixels * self.bounds.width / self.image_size.width
    }

    /// Keeps the canvas center over the image, so it stays reachable
    fn manual(self, scale: f32, center: Point) -> Option<Navigation> {
        let center = Point::new(
            center.x.clamp(0.0, self.image_size.width),
            center.y.clamp(0.0, self.image_size.height),
        );
        [scale, center.x, center.y]
            .into_iter()
            .all(f32::is_finite)
            .then_some(Navigation::Manual { scale, center })
    }
}

pub fn rectangle_between(start: Point, end: Point) -> Option<Rectangle> {
    let left = start.x.min(end.x);
    let top = start.y.min(end.y);
    let width = start.x.max(end.x) - left;
    let height = start.y.max(end.y) - top;

    (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
        .then(|| Rectangle::new(Point::new(left, top), Size::new(width, height)))
}

/// Moves a `point` by `offset`, both in the same coordinate space
pub const fn translate(point: Point, offset: Vector) -> Point {
    Point::new(point.x + offset.x, point.y + offset.y)
}

/// Moves a `rectangle` by `offset` without resizing it
pub const fn translate_rectangle(rectangle: Rectangle, offset: Vector) -> Rectangle {
    Rectangle {
        x: rectangle.x + offset.x,
        y: rectangle.y + offset.y,
        ..rectangle
    }
}

fn place(image_size: Size, canvas: Size, scale: f32, center: Point) -> Rectangle {
    Rectangle::new(
        Point::new(
            (-center.x).mul_add(scale, canvas.width / 2.0),
            (-center.y).mul_add(scale, canvas.height / 2.0),
        ),
        Size::new(image_size.width * scale, image_size.height * scale),
    )
}

fn snap(origin: f32, scale: f32) -> f32 {
    (origin / scale).trunc() * scale
}

fn is_positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some, assert_some_eq};
    use rstest::rstest;

    #[rstest]
    #[case(
        Size::new(400.0, 400.0),
        Rectangle::new(Point::new(0.0, 100.0), Size::new(400.0, 200.0))
    )]
    #[case(Size::new(600.0, 300.0), Rectangle::with_size(Size::new(600.0, 300.0)))]
    #[case(
        Size::new(100.0, 300.0),
        Rectangle::new(Point::new(0.0, 125.0), Size::new(100.0, 50.0))
    )]
    fn fits_the_image_without_distorting_it(#[case] canvas: Size, #[case] expected: Rectangle) {
        let viewport = assert_some!(Viewport::new(
            Size::new(200.0, 100.0),
            Rectangle::with_size(canvas)
        ));
        assert_eq!(viewport.bounds, expected);
    }

    #[test]
    fn excludes_letterboxing_and_clamps_drag_endpoints() {
        let viewport = assert_some!(Viewport::new(
            Size::new(200.0, 100.0),
            Rectangle::with_size(Size::new(400.0, 400.0))
        ));

        assert_none!(viewport.image_position(Point::new(100.0, 50.0)));
        assert_eq!(
            viewport.clamped_image_position(Point::new(-20.0, 500.0)),
            Some(Point::new(0.0, 100.0))
        );
        assert_none!(viewport.clamped_image_position(Point::new(f32::NAN, 0.0)));
    }

    #[rstest]
    #[case(Size::ZERO)]
    #[case(Size::new(0.0, 20.0))]
    #[case(Size::new(20.0, -1.0))]
    #[case(Size::new(f32::NAN, 20.0))]
    #[case(Size::INFINITE)]
    fn rejects_unusable_dimensions(#[case] invalid: Size) {
        assert_none!(Viewport::new(
            invalid,
            Rectangle::with_size(Size::new(400.0, 400.0))
        ));
        assert_none!(Viewport::new(
            Size::new(200.0, 100.0),
            Rectangle::with_size(invalid)
        ));
    }

    #[test]
    fn maps_the_same_image_rectangle_after_resizing() {
        let rectangle = Rectangle::new(Point::new(50.0, 25.0), Size::new(100.0, 50.0));

        for canvas in [Size::new(400.0, 400.0), Size::new(600.0, 300.0)] {
            let viewport = assert_some!(Viewport::new(
                Size::new(200.0, 100.0),
                Rectangle::with_size(canvas)
            ));
            let displayed = viewport.canvas_rectangle(rectangle);
            let start = assert_some!(viewport.image_position(displayed.position()));
            let end = assert_some!(viewport.image_position(Point::new(
                displayed.x + displayed.width,
                displayed.y + displayed.height,
            )));

            assert_eq!(rectangle_between(start, end), Some(rectangle));
        }
    }

    #[rstest]
    #[case(Point::new(10.0, 20.0), Point::new(50.0, 70.0))]
    #[case(Point::new(50.0, 70.0), Point::new(10.0, 20.0))]
    fn normalizes_every_drag_direction(#[case] start: Point, #[case] end: Point) {
        assert_eq!(
            rectangle_between(start, end),
            Some(Rectangle::new(
                Point::new(10.0, 20.0),
                Size::new(40.0, 50.0)
            ))
        );
    }

    #[test]
    fn resizing_refits_the_fit_view_and_keeps_a_manual_scale_and_center() {
        let image = Size::new(200.0, 100.0);
        let center = Point::new(50.0, 25.0);
        let manual = Navigation::Manual { scale: 4.0, center };

        // The second fit lands at y = 9, not 10, after snapping to whole image pixels
        for (canvas, fitted) in [
            (
                Size::new(400.0, 400.0),
                Rectangle::new(Point::new(0.0, 100.0), Size::new(400.0, 200.0)),
            ),
            (
                Size::new(600.0, 320.0),
                Rectangle::new(Point::new(0.0, 9.0), Size::new(600.0, 300.0)),
            ),
        ] {
            let canvas = Rectangle::with_size(canvas);
            let fit = assert_some!(Viewport::navigated(image, canvas, Navigation::Fit));
            assert_eq!(fit.bounds, fitted);

            let viewport = assert_some!(Viewport::navigated(image, canvas, manual));
            assert_eq!(viewport.canvas_length(1.0), 4.0);
            assert_some_eq!(viewport.image_position(canvas.center()), center);
        }
    }

    #[test]
    fn requests_the_image_where_the_renderer_draws_it() {
        let image = Size::new(200.0, 100.0);
        let canvas = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));

        // Origins on both sides of the window edge, magnified and reduced
        for (scale, center) in [
            (6.5, Point::new(20.3, 70.1)),
            (6.5, Point::new(180.7, 30.9)),
            (0.75, Point::new(13.0, 7.0)),
        ] {
            let navigation = Navigation::Manual { scale, center };
            let viewport = assert_some!(Viewport::navigated(image, canvas, navigation));
            let placement = viewport.placement();

            for (requested, displayed, offset) in [
                (placement.x, viewport.bounds.x, canvas.x),
                (placement.y, viewport.bounds.y, canvas.y),
            ] {
                // Tiny-skia's raster origin
                let drawn = ((offset + requested) / scale)
                    .trunc()
                    .mul_add(scale, -offset);
                assert!((drawn - displayed).abs() < 1e-3, "{drawn} != {displayed}");
                assert!((requested - displayed).abs() <= 0.5);
            }
        }
    }

    #[test]
    fn rejects_drags_without_area() {
        assert_none!(rectangle_between(Point::ORIGIN, Point::ORIGIN));
        assert_none!(rectangle_between(Point::ORIGIN, Point::new(10.0, 0.0)));
        assert_none!(rectangle_between(Point::ORIGIN, Point::new(0.0, 10.0)));
    }
}
