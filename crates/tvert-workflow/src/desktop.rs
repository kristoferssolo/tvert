use crate::Error;
use std::{cmp::Ordering, num::NonZeroU32, ops::Range};
use tvert_image::{CapturedImage, PixelSize};
use tvert_wayland::{LogicalRect, RegionBounds};

/// A captured output, oriented as displayed, and its logical desktop area
#[derive(Debug)]
pub struct Screen {
    pub name: String,
    pub area: LogicalRect,
    pub image: CapturedImage,
}

/// A composed desktop image and the logical-to-pixel mapping that placed its outputs
#[derive(Debug)]
pub struct Desktop {
    image: CapturedImage,
    x: Axis,
    y: Axis,
}

/// Combines captured outputs into one image of the desktop layout.
///
/// The densest output sets the image's pixels per logical unit, so it is
/// copied without resampling and other outputs are scaled to match. Logical
/// edges map to the nearest image edge, rounding halves up, so outputs that
/// share an edge meet without seams or overlap. Uncovered areas stay transparent.
///
/// Where outputs overlap, the denser output is visible. Among outputs with
/// equal density, the one whose name sorts first is visible.
///
/// # Errors
///
/// Returns [`Error::NoOutputs`] if `screens` is empty, [`Error::DesktopTooLarge`]
/// if the layout cannot be mapped to image coordinates, or [`Error::Compose`]
/// if allocating or painting the combined image fails.
pub fn compose(mut screens: Vec<Screen>) -> Result<Desktop, Error> {
    screens.sort_by(|left, right| {
        left.densities()
            .cmp(&right.densities())
            .then_with(|| right.name.cmp(&left.name))
    });
    let densest = screens.last().ok_or(Error::NoOutputs)?;
    let (x_density, y_density) = densest.densities();

    let x = Axis::new(screens.iter().map(|s| horizontal(&s.area)), x_density)?;
    let y = Axis::new(screens.iter().map(|s| vertical(&s.area)), y_density)?;
    let size = PixelSize::try_from((x.canvas_len()?, y.canvas_len()?)).map_err(Error::Compose)?;
    let mut canvas = CapturedImage::transparent(size).map_err(Error::Compose)?;

    for screen in &screens {
        let columns = x.pixels(horizontal(&screen.area))?;
        let rows = y.pixels(vertical(&screen.area))?;
        canvas
            .paint(&screen.image, columns, rows)
            .map_err(Error::Compose)?;
    }

    Ok(Desktop {
        image: canvas,
        x,
        y,
    })
}

impl Screen {
    const fn densities(&self) -> (Density, Density) {
        let size = self.image.size();
        (
            Density {
                pixels: size.width(),
                logical: self.area.width,
            },
            Density {
                pixels: size.height(),
                logical: self.area.height,
            },
        )
    }
}

impl Desktop {
    pub fn into_image(self) -> CapturedImage {
        self.image
    }

    pub fn crop(&self, bounds: RegionBounds) -> Result<CapturedImage, Error> {
        let size = self.image.size();
        let columns = self
            .x
            .covering(bounds.left()..bounds.right(), size.width())?;
        let rows = self
            .y
            .covering(bounds.top()..bounds.bottom(), size.height())?;
        self.image.crop(columns, rows).map_err(Error::Crop)
    }
}

fn horizontal(area: &LogicalRect) -> Range<i64> {
    span(area.x, area.width)
}

fn vertical(area: &LogicalRect) -> Range<i64> {
    span(area.y, area.height)
}

fn span(start: i32, len: NonZeroU32) -> Range<i64> {
    let start = i64::from(start);
    start..start.saturating_add(i64::from(len.get()))
}

/// Image pixels per logical unit along one axis, as an exact ratio
#[derive(Debug, Clone, Copy)]
struct Density {
    pixels: NonZeroU32,
    logical: NonZeroU32,
}

impl Ord for Density {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = u64::from(self.pixels.get()).saturating_mul(u64::from(other.logical.get()));
        let right = u64::from(other.pixels.get()).saturating_mul(u64::from(self.logical.get()));
        left.cmp(&right)
    }
}

impl PartialOrd for Density {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Density {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Density {}

/// Maps one logical desktop axis onto image pixels
#[derive(Debug)]
struct Axis {
    bounds: Range<i64>,
    density: Density,
}

impl Axis {
    fn new(spans: impl Iterator<Item = Range<i64>>, density: Density) -> Result<Self, Error> {
        let bounds = spans
            .reduce(|a, b| a.start.min(b.start)..a.end.max(b.end))
            .ok_or(Error::NoOutputs)?;

        Ok(Self { bounds, density })
    }

    fn canvas_len(&self) -> Result<u32, Error> {
        let len = self.edge(self.bounds.end)?;
        u32::try_from(len).map_err(|_| Error::DesktopTooLarge)
    }

    fn pixels(&self, span: Range<i64>) -> Result<Range<usize>, Error> {
        Ok(self.edge(span.start)?..self.edge(span.end)?)
    }

    fn edge(&self, coordinate: i64) -> Result<usize, Error> {
        let pixels = u64::from(self.density.pixels.get());
        let logical = u64::from(self.density.logical.get());

        coordinate
            .checked_sub(self.bounds.start)
            .and_then(|offset| u64::try_from(offset).ok())
            .and_then(|offset| offset.checked_mul(pixels))
            .and_then(|scaled| scaled.checked_add(logical.checked_div(2)?))
            .and_then(|scaled| scaled.checked_div(logical))
            .and_then(|edge| usize::try_from(edge).ok())
            .ok_or(Error::DesktopTooLarge)
    }

    fn covering(&self, span: Range<f64>, limit: NonZeroU32) -> Result<Range<usize>, Error> {
        let start = self.position(span.start)? - 1.0;
        let end = self.position(span.end)?;
        let start = first_edge(limit.get(), |edge| edge > start);
        let end = first_edge(limit.get(), |edge| edge >= end);
        let index = |edge| usize::try_from(edge).map_err(|_| Error::DesktopTooLarge);

        Ok(index(start)?..index(end)?)
    }

    fn position(&self, coordinate: f64) -> Result<f64, Error> {
        let origin = i32::try_from(self.bounds.start).map_err(|_| Error::DesktopTooLarge)?;
        let pixels = f64::from(self.density.pixels.get());
        let logical = f64::from(self.density.logical.get());

        Ok((coordinate - f64::from(origin)) * pixels / logical)
    }
}

fn first_edge(limit: u32, reached: impl Fn(f64) -> bool) -> u32 {
    let (mut low, mut high) = (0, limit);
    while low < high {
        let middle = low.midpoint(high);
        if reached(f64::from(middle)) {
            high = middle;
        } else {
            low = middle.saturating_add(1);
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some};

    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const CLEAR: [u8; 4] = [0; 4];

    fn screen(name: &str, [x, y, width, height]: [i32; 4], image: CapturedImage) -> Screen {
        let len = |value| assert_some!(u32::try_from(value).ok().and_then(NonZeroU32::new));
        let area = LogicalRect {
            x,
            y,
            width: len(width),
            height: len(height),
        };

        Screen {
            name: name.to_owned(),
            area,
            image,
        }
    }

    fn image(width: u32, height: u32, pixels: &[[u8; 4]]) -> CapturedImage {
        let size = assert_ok!(PixelSize::try_from((width, height)));
        assert_ok!(CapturedImage::new(size, pixels.concat()))
    }

    fn filled(width: u32, height: u32, pixel: [u8; 4]) -> CapturedImage {
        let len = assert_ok!(usize::try_from(assert_some!(width.checked_mul(height))));
        image(width, height, &vec![pixel; len])
    }

    fn pixels(image: &CapturedImage) -> Vec<[u8; 4]> {
        image.rgba().as_chunks::<4>().0.to_vec()
    }

    fn bounds([left, top, right, bottom]: [f64; 4]) -> RegionBounds {
        assert_some!(RegionBounds::new(left, top, right, bottom))
    }

    // Desktop, one logical unit per character:
    //
    // -2 -1  0
    //  L  L  .   y = 0
    //  .  .  B   y = 1
    //  .  .  B   y = 2
    #[test]
    fn places_outputs_by_logical_position_with_transparent_gaps() {
        let screens = vec![
            screen("left", [-2, 0, 2, 1], filled(2, 1, RED)),
            screen("below", [0, 1, 1, 2], filled(1, 2, GREEN)),
        ];

        let desktop = assert_ok!(compose(screens)).into_image();

        assert_eq!(desktop.size(), assert_ok!(PixelSize::try_from((3, 3))));
        assert_eq!(
            pixels(&desktop),
            [RED, RED, CLEAR, CLEAR, CLEAR, GREEN, CLEAR, CLEAR, GREEN]
        );
    }

    // A 3x3 capture of a 2x2 logical output sets 1.5 pixels per unit. The
    // portrait output covers logical x = 2..3 at one pixel per unit, so it
    // maps to columns 3..5, since 4.5 rounds up, and is scaled from 1x3 to 2x5.
    #[test]
    fn scales_outputs_to_the_densest_output_without_seams() {
        let detail = (1..=9).map(|value| [value, 0, 0, 255]).collect::<Vec<_>>();
        let screens = vec![
            screen("portrait", [2, 0, 1, 3], filled(1, 3, BLUE)),
            screen("dense", [0, 0, 2, 2], image(3, 3, &detail)),
        ];

        let desktop = assert_ok!(compose(screens)).into_image();

        assert_eq!(desktop.size(), assert_ok!(PixelSize::try_from((5, 5))));
        let pixels = pixels(&desktop);
        let (rows, _) = pixels.as_chunks::<5>();
        let (detail, _) = detail.as_chunks::<3>();
        for (row, detail) in rows.iter().zip(detail) {
            assert_eq!(&row[..3], detail);
        }
        for row in rows {
            assert_eq!(row[3..], [BLUE; 2]);
        }
        for row in &rows[3..] {
            assert_eq!(row[..3], [CLEAR; 3]);
        }
    }

    #[test]
    fn overlapping_outputs_show_the_denser_then_first_named_output() {
        let mirrored = vec![
            screen("HDMI-A-1", [0, 0, 1, 1], filled(2, 2, RED)),
            screen("eDP-1", [0, 0, 1, 1], filled(1, 1, BLUE)),
        ];
        let identical = vec![
            screen("DP-2", [0, 0, 1, 1], filled(1, 1, RED)),
            screen("DP-1", [0, 0, 1, 1], filled(1, 1, GREEN)),
        ];

        let mirrored = assert_ok!(compose(mirrored)).into_image();
        let identical = assert_ok!(compose(identical)).into_image();
        assert_eq!(pixels(&mirrored), [RED; 4]);
        assert_eq!(pixels(&identical), [GREEN]);
    }

    #[test]
    fn crops_a_region_spanning_outputs_left_of_the_origin() {
        let left = [[1, 0, 0, 255], [2, 0, 0, 128]];
        let right = [[3, 0, 0, 64], [4, 0, 0, 255]];
        let screens = vec![
            screen("left", [-2, 0, 2, 1], image(2, 1, &left)),
            screen("right", [0, 0, 2, 1], image(2, 1, &right)),
        ];

        let desktop = assert_ok!(compose(screens));
        let region = assert_ok!(desktop.crop(bounds([-1.0, 0.25, 1.0, 0.75])));

        assert_eq!(region.size(), assert_ok!(PixelSize::try_from((2, 1))));
        assert_eq!(pixels(&region), [left[1], right[0]]);
    }

    // The dense output sets 1.5 pixels per unit on a 5x5 image, as in
    // scales_outputs_to_the_densest_output_without_seams. Logical x = 1.8..2.2
    // spans pixels 2.7..3.3, so the crop covers columns 2..4. Rounding the
    // bounds to logical units, rounding pixels to nearest, or deriving the
    // density from the 5-pixel width for 3 logical units would differ.
    #[test]
    fn crops_fractional_bounds_outward_at_the_composed_density() {
        let detail = (1..=9).map(|value| [value, 0, 0, 255]).collect::<Vec<_>>();
        let screens = vec![
            screen("portrait", [2, 0, 1, 3], filled(1, 3, BLUE)),
            screen("dense", [0, 0, 2, 2], image(3, 3, &detail)),
        ];

        let desktop = assert_ok!(compose(screens));
        let region = assert_ok!(desktop.crop(bounds([1.8, 0.5, 2.2, 1.5])));

        assert_eq!(region.size(), assert_ok!(PixelSize::try_from((2, 3))));
        let (detail, _) = detail.as_chunks::<3>();
        let expected = detail.iter().flat_map(|row| [row[2], BLUE]);
        assert_eq!(pixels(&region), expected.collect::<Vec<_>>());
    }

    #[test]
    fn clips_bounds_to_the_desktop_and_rejects_bounds_outside_it() {
        let layout = || {
            vec![
                screen("left", [-2, 0, 2, 1], filled(2, 1, RED)),
                screen("below", [0, 1, 1, 2], filled(1, 2, GREEN)),
            ]
        };
        let whole = assert_ok!(compose(layout())).into_image();
        let desktop = assert_ok!(compose(layout()));

        let region = assert_ok!(desktop.crop(bounds([-9.5, -3.0, 4.0, 7.25])));
        assert_eq!(region.size(), whole.size());
        assert_eq!(region.rgba(), whole.rgba());

        let error = assert_err!(desktop.crop(bounds([1.5, 0.0, 3.0, 2.0])));
        assert!(matches!(error, Error::Crop(_)), "{error:?}");
    }
}
