use iced::{ContentFit, Point, Rectangle, Size};

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub bounds: Rectangle,
    image_size: Size,
}

impl Viewport {
    pub fn new(image_size: Size, canvas_size: Size) -> Option<Self> {
        if ![
            image_size.width,
            image_size.height,
            canvas_size.width,
            canvas_size.height,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0)
        {
            return None;
        }

        let fitted = ContentFit::Contain.fit(image_size, canvas_size);
        if ![fitted.width, fitted.height]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        {
            return None;
        }

        Some(Self {
            bounds: Rectangle::new(
                Point::new(
                    (canvas_size.width - fitted.width) / 2.0,
                    (canvas_size.height - fitted.height) / 2.0,
                ),
                fitted,
            ),
            image_size,
        })
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

    pub fn canvas_rectangle(self, rectangle: Rectangle) -> Rectangle {
        Rectangle {
            x: (rectangle.x / self.image_size.width).mul_add(self.bounds.width, self.bounds.x),
            y: (rectangle.y / self.image_size.height).mul_add(self.bounds.height, self.bounds.y),
            width: rectangle.width / self.image_size.width * self.bounds.width,
            height: rectangle.height / self.image_size.height * self.bounds.height,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
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
        let viewport = assert_some!(Viewport::new(Size::new(200.0, 100.0), canvas));
        assert_eq!(viewport.bounds, expected);
    }

    #[test]
    fn excludes_letterboxing_and_clamps_drag_endpoints() {
        let viewport = assert_some!(Viewport::new(
            Size::new(200.0, 100.0),
            Size::new(400.0, 400.0),
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
        assert_none!(Viewport::new(invalid, Size::new(400.0, 400.0)));
        assert_none!(Viewport::new(Size::new(200.0, 100.0), invalid));
    }

    #[test]
    fn maps_the_same_image_rectangle_after_resizing() {
        let rectangle = Rectangle::new(Point::new(50.0, 25.0), Size::new(100.0, 50.0));

        for canvas in [Size::new(400.0, 400.0), Size::new(600.0, 300.0)] {
            let viewport = assert_some!(Viewport::new(Size::new(200.0, 100.0), canvas));
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
    #[case(Point::new(50.0, 20.0), Point::new(10.0, 70.0))]
    #[case(Point::new(10.0, 70.0), Point::new(50.0, 20.0))]
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
    fn rejects_drags_without_area() {
        assert_none!(rectangle_between(Point::ORIGIN, Point::ORIGIN));
        assert_none!(rectangle_between(Point::ORIGIN, Point::new(10.0, 0.0)));
        assert_none!(rectangle_between(Point::ORIGIN, Point::new(0.0, 10.0)));
    }
}
