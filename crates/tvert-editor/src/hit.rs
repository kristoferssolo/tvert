use iced::{
    Point, Rectangle, Size,
    widget::canvas::{
        Path,
        path::lyon_path::{PathEvent, geom::LineSegment, math, traits::PathIterator},
    },
};

/// Reach beyond visible geometry, in viewport pixels
pub const TOLERANCE: f32 = 6.0;

/// Maximum distance between a curve and its flattened segments, in viewport pixels
const FLATTENING: f32 = 0.25;

/// Whether `point` is within reach of a path stroked `width` canvas units wide
pub fn stroke(path: &Path, point: Point, width: f32) -> bool {
    let point = math::point(point.x, point.y);
    let reach = width.mul_add(0.5, TOLERANCE);

    segments(path, false).any(|segment| segment.distance_to_point(point) <= reach)
}

/// Whether `point` is inside a filled path or within reach of its edge.
///
/// Uses the nonzero rule that iced fills with by default.
pub fn fill(path: &Path, point: Point) -> bool {
    stroke(path, point, 0.0) || winding(path, math::point(point.x, point.y)) != 0
}

/// Returns the smallest rectangle containing the flattened path
pub fn extent(path: &Path) -> Option<Rectangle> {
    segments(path, false)
        .flat_map(|LineSegment { from, to }| [from, to])
        .map(|point| Rectangle::new(Point::new(point.x, point.y), Size::ZERO))
        .reduce(|first, second| first.union(&second))
}

/// Flattens `path` into line segments.
///
/// `close_all` adds the closing edge of open subpaths, as filling does.
fn segments(path: &Path, close_all: bool) -> impl Iterator<Item = LineSegment<f32>> + '_ {
    path.raw()
        .iter()
        .flattened(FLATTENING)
        .filter_map(move |event| match event {
            PathEvent::Line { from, to } => Some(LineSegment { from, to }),
            PathEvent::End { last, first, close } if close || close_all => Some(LineSegment {
                from: last,
                to: first,
            }),
            _ => None,
        })
}

/// Sums the signed edge crossings of a horizontal ray from `point`
fn winding(path: &Path, point: math::Point) -> i32 {
    segments(path, true)
        .map(|LineSegment { from, to }| {
            let side =
                (to.x - from.x).mul_add(point.y - from.y, -(point.x - from.x) * (to.y - from.y));

            match (from.y <= point.y, to.y <= point.y) {
                (true, false) if side > 0.0 => 1,
                (false, true) if side < 0.0 => -1,
                _ => 0,
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;
    use rstest::rstest;

    #[rstest]
    #[case::closed(Path::rectangle(Point::new(10.0, 20.0), Size::new(30.0, 40.0)))]
    #[case::open(Path::new(|builder| {
        builder.move_to(Point::new(10.0, 60.0));
        builder.line_to(Point::new(10.0, 20.0));
        builder.line_to(Point::new(40.0, 20.0));
        builder.line_to(Point::new(40.0, 60.0));
    }))]
    fn extent_covers_axis_aligned_edges(#[case] path: Path) {
        assert_some_eq!(
            extent(&path),
            Rectangle::new(Point::new(10.0, 20.0), Size::new(30.0, 40.0))
        );
    }
}
