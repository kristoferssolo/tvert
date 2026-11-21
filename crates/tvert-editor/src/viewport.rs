use iced::{Padding, Point, Rectangle, Size, Vector};

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
    /// The shown part of the image in captured-image coordinates
    area: Rectangle,
    canvas: Rectangle,
    view: Rectangle,
    fit: f32,
    scale: f32,
    center: Point,
}

impl Viewport {
    /// Fits the whole image in `canvas`
    pub fn new(image_size: Size, canvas: Rectangle) -> Option<Self> {
        Self::navigated(image_size, None, canvas, Padding::ZERO, Navigation::Fit)
    }

    /// Places the image in `canvas`, given in window coordinates, as `navigation` requests.
    ///
    /// `area` limits the shown part of the image, in captured-image
    /// coordinates; `None` shows all of it. Fitting, positions, and the
    /// reachable view center stay within that area. Callers hide what lies
    /// outside [`Self::visible`].
    ///
    /// Returns `None` for empty or non-finite sizes and placements.
    pub fn navigated(
        image_size: Size,
        area: Option<Rectangle>,
        canvas: Rectangle,
        padding: Padding,
        navigation: Navigation,
    ) -> Option<Self> {
        let view = Rectangle::with_size(canvas.size()).shrink(padding);
        let area = area.unwrap_or_else(|| Rectangle::with_size(image_size));
        if ![
            image_size.width,
            image_size.height,
            area.width,
            area.height,
            canvas.width,
            canvas.height,
        ]
        .into_iter()
        .all(is_positive)
            || !area.x.is_finite()
            || !area.y.is_finite()
        {
            return None;
        }

        let fit = (canvas.width / area.width).min(canvas.height / area.height);
        let (scale, center) = match navigation {
            Navigation::Fit => (fit, area.center()),
            Navigation::Manual { scale, center } => (scale, center),
        };
        let placement = place(image_size, view, scale, center);
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
            area,
            canvas,
            view,
            fit,
            scale,
            center,
        })
    }

    /// The shown part of the image in captured-image coordinates
    pub const fn area(self) -> Rectangle {
        self.area
    }

    /// The shown part of the image in canvas coordinates
    pub fn visible(self) -> Rectangle {
        self.canvas_rectangle(self.area)
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
                (anchor.x - self.view.center_x()).mul_add(shift, self.center.x),
                (anchor.y - self.view.center_y()).mul_add(shift, self.center.y),
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

    /// Maps a canvas `point` within the shown area to image pixels
    pub fn image_position(self, point: Point) -> Option<Point> {
        if !self.visible().contains(point) {
            return None;
        }
        self.clamped_image_position(point)
    }

    /// Maps a canvas `point` to image pixels, clamped to the shown area
    pub fn clamped_image_position(self, point: Point) -> Option<Point> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }

        Some(Point::new(
            ((point.x - self.bounds.x) / self.bounds.width * self.image_size.width)
                .clamp(self.area.x, self.area.x + self.area.width),
            ((point.y - self.bounds.y) / self.bounds.height * self.image_size.height)
                .clamp(self.area.y, self.area.y + self.area.height),
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

    /// Keeps the view center over the shown area, so it stays reachable
    fn manual(self, scale: f32, center: Point) -> Option<Navigation> {
        let center = Point::new(
            center.x.clamp(self.area.x, self.area.x + self.area.width),
            center.y.clamp(self.area.y, self.area.y + self.area.height),
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

fn place(image_size: Size, view: Rectangle, scale: f32, center: Point) -> Rectangle {
    Rectangle::new(
        Point::new(
            (-center.x).mul_add(scale, view.center_x()),
            (-center.y).mul_add(scale, view.center_y()),
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

    const IMAGE: Size = Size::new(200.0, 100.0);
    const CANVAS: Rectangle = Rectangle::new(Point::ORIGIN, Size::new(400.0, 400.0));
    /// Fits at four canvas pixels per image pixel, shown at (0, 100)–(400, 300)
    const AREA: Rectangle = Rectangle::new(Point::new(50.0, 25.0), Size::new(100.0, 50.0));

    fn viewport(navigation: Navigation) -> Viewport {
        assert_some!(Viewport::navigated(
            IMAGE,
            Some(AREA),
            CANVAS,
            Padding::ZERO,
            navigation
        ))
    }

    #[test]
    fn fits_the_shown_area_instead_of_the_image() {
        let viewport = viewport(Navigation::Fit);

        assert_eq!(
            viewport.visible(),
            Rectangle::new(Point::new(0.0, 100.0), Size::new(400.0, 200.0))
        );
        assert_eq!(viewport.canvas_length(1.0), 4.0);
    }

    #[test]
    fn positions_stay_within_the_shown_area() {
        let viewport = viewport(Navigation::Fit);

        assert_some_eq!(
            viewport.image_position(Point::new(200.0, 200.0)),
            Point::new(100.0, 50.0)
        );
        assert_none!(viewport.image_position(Point::new(200.0, 50.0)));
        assert_some_eq!(
            viewport.clamped_image_position(Point::new(-40.0, 380.0)),
            Point::new(50.0, 75.0)
        );
    }

    #[test]
    fn panning_keeps_the_center_over_the_shown_area() {
        let navigation = assert_some!(viewport(Navigation::Fit).pan(Vector::new(4000.0, -4000.0)));

        assert_eq!(
            navigation,
            Navigation::Manual {
                scale: 4.0,
                center: Point::new(50.0, 75.0),
            }
        );
    }
}
