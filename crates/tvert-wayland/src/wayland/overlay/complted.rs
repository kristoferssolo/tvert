use crate::{
    Error, FrozenOutput, LogicalRect, RegionSelection,
    wayland::overlay::{crop, selection::Gesture, size::SurfaceSize},
};
use tvert_image::CapturedImage;

/// Keeps completion data available after selector surfaces have been destroyed.
#[derive(Debug)]
pub(super) struct Selected {
    pub outputs: Vec<FrozenOutput>,
    pub gesture: Gesture,
    pub size: SurfaceSize,
    pub area: LogicalRect,
}

impl Selected {
    pub fn into_screen(self) -> Result<CapturedImage, Error> {
        let origin = self.gesture.origin();
        let frozen = self
            .outputs
            .into_iter()
            .find(|frozen| frozen.output.id == origin)
            .ok_or(Error::UnknownOutput(origin))?;
        Ok(frozen.into_parts()?.1)
    }

    pub fn into_region(self) -> Result<Option<CapturedImage>, Error> {
        let origin = self.gesture.origin();
        let frozen = self
            .outputs
            .into_iter()
            .find(|frozen| frozen.output.id == origin)
            .ok_or(Error::UnknownOutput(origin))?;
        let Some(rectangle) = self.gesture.rectangle(self.size, self.area) else {
            return Ok(None);
        };
        crop::selection(&frozen.image, rectangle, self.size, frozen.orientation)
    }

    pub fn into_desktop_region(self) -> Option<RegionSelection> {
        Some(RegionSelection {
            outputs: self.outputs,
            bounds: self.gesture.bounds()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Output, OutputId, PhysicalSize,
        wayland::overlay::{
            DesktopPosition, SelectionState, SuggestedSurfaceSize, SurfacePosition,
        },
    };
    use claims::{assert_none, assert_ok, assert_some};
    use tvert_image::{ImageTransform, PixelSize};

    fn selected(start: (f64, f64), end: (f64, f64)) -> Selected {
        let size = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 4,
            height: 2,
        }));
        let output = |id, x| Output {
            id: OutputId(id),
            name: format!("DP-{id}"),
            size: PhysicalSize {
                width: 2,
                height: 1,
            },
            scale: 1,
            logical: Some(LogicalRect {
                x,
                ..size.at_origin()
            }),
        };
        let outputs = [output(1, -4), output(2, 0)]
            .into_iter()
            .map(|output| FrozenOutput {
                output,
                image: assert_ok!(CapturedImage::new(
                    assert_ok!(PixelSize::try_from((2, 1))),
                    vec![10, 20, 30, 128, 40, 50, 60, 255],
                )),
                orientation: ImageTransform::Rotate180,
            })
            .collect();
        let area = LogicalRect {
            x: -4,
            ..size.at_origin()
        };
        let desktop = |(x, y)| {
            assert_some!(DesktopPosition::from_surface(
                assert_some!(SurfacePosition::new(x, y)),
                size,
                area,
            ))
        };
        let mut selection = SelectionState::Idle;
        selection.begin(OutputId(1), desktop(start));
        selection.update(desktop(end));
        selection.finish();
        Selected {
            outputs,
            gesture: assert_some!(selection.take_complete()),
            size,
            area,
        }
    }

    #[test]
    fn monitor_completion_returns_the_oriented_original_even_for_a_click() {
        let image = assert_ok!(selected((0.5, 0.5), (0.5, 0.5)).into_screen());
        assert_eq!(image.rgba(), [40, 50, 60, 255, 10, 20, 30, 128]);
    }

    #[test]
    fn named_completion_crops_before_orienting_and_rejects_empty_drags() {
        let image = assert_some!(assert_ok!(selected((2.5, 0.5), (3.5, 1.5)).into_region()));
        assert_eq!(image.size(), assert_ok!(PixelSize::try_from((1, 1))));
        assert_eq!(image.rgba(), [10, 20, 30, 128]);
        assert_none!(assert_ok!(selected((0.5, 0.5), (0.5, 0.5)).into_region()));
    }

    #[test]
    fn desktop_completion_preserves_all_raw_captures_and_fractional_bounds() {
        let region = assert_some!(selected((6.5, 1.5), (0.5, 0.5)).into_desktop_region());
        let bounds = region.bounds;
        assert_eq!(
            [bounds.left(), bounds.top(), bounds.right(), bounds.bottom()].map(f64::to_bits),
            [-3.5_f64, 0.5, 2.5, 1.5].map(f64::to_bits),
        );
        assert_eq!(region.outputs.len(), 2);
        for (capture, id) in region.outputs.iter().zip([OutputId(1), OutputId(2)]) {
            assert_eq!(capture.output.id, id);
            assert_eq!(capture.orientation, ImageTransform::Rotate180);
            assert_eq!(capture.image.rgba(), [10, 20, 30, 128, 40, 50, 60, 255]);
        }
        assert_none!(selected((0.5, 0.5), (0.5, 0.5)).into_desktop_region());
    }
}
