use crate::{
    Error, FrozenOutput, LogicalRect, OutputId, Wayland,
    wayland::overlay::{
        SelectionState, SurfaceRect, complted::Selected, geometry::PixelRect, selection::Gesture,
        size::SurfaceSize, state::OverlayStatus,
    },
};
use tracing::debug;
use wayland_client::protocol::wl_output::Transform;

#[derive(Debug, Clone, Copy)]
pub(super) enum SelectionTarget {
    Screen,
    Region(OutputId),
    DesktopRegion,
}

struct ScreenPreview {
    frozen: FrozenOutput,
    transform: Transform,
    displayed: Option<(SurfaceSize, Option<SurfaceRect>)>,
}

impl SelectionTarget {
    fn rectangle(
        self,
        selection: SelectionState,
        size: SurfaceSize,
        area: LogicalRect,
        hovered: bool,
    ) -> Option<SurfaceRect> {
        match self {
            Self::Screen => hovered.then(|| SurfaceRect::full(size)),
            Self::Region(_) | Self::DesktopRegion => selection.rectangle(size, area),
        }
    }
}

impl Wayland {
    fn prepare_selection(&mut self, target: SelectionTarget) -> Result<Vec<ScreenPreview>, Error> {
        let outputs = match target {
            SelectionTarget::Screen | SelectionTarget::DesktopRegion => self.freeze_outputs()?,
            SelectionTarget::Region(output_id) => {
                let image = self.capture_buffer(output_id)?;
                let output = self
                    .state
                    .outputs
                    .get(&output_id)
                    .ok_or(Error::UnknownOutput(output_id))?
                    .clone();
                let orientation = self
                    .state
                    .output_states
                    .get(&output_id)
                    .ok_or(Error::UnknownOutput(output_id))?
                    .displayed_transform()?;
                vec![FrozenOutput {
                    output,
                    image,
                    orientation,
                }]
            }
        };
        if matches!(target, SelectionTarget::DesktopRegion) {
            require_placements(&outputs)?;
        }
        let screens = outputs
            .into_iter()
            .map(|frozen| {
                let transform = self
                    .state
                    .output_states
                    .get(&frozen.output.id)
                    .ok_or(Error::UnknownOutput(frozen.output.id))?
                    .transform()?;
                Ok(ScreenPreview {
                    frozen,
                    transform,
                    displayed: None,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // Capture every output before any selector surfacce is mapped
        for screen in &screens {
            self.prepare_selection_overlay(screen.frozen.output.id)?;
        }
        Ok(screens)
    }

    pub(super) fn run_selection(
        &mut self,
        target: SelectionTarget,
    ) -> Result<Option<Selected>, Error> {
        let mut screens = self.prepare_selection(target)?;
        loop {
            if matches!(target, SelectionTarget::Screen) {
                self.validate_frozen_ouputs(screens.iter().map(|screen| &screen.frozen))?;
            }
            for screen in &screens {
                let Some(overlay) = self.state.overlays.get(&screen.frozen.output.id) else {
                    return Ok(None);
                };
                if matches!(
                    overlay.status,
                    OverlayStatus::Closed | OverlayStatus::Cancelled
                ) {
                    return Ok(None);
                }
                overlay.status.size()?;
            }

            if let Some(gesture) = self.state.selection.take_complete() {
                let Some((size, area)) = self.state.overlay_area(gesture.origin()) else {
                    continue;
                };
                if !matches!(target, SelectionTarget::Screen)
                    && !self.has_region(gesture, &screens)?
                {
                    debug!("selection has no area on the output");
                    continue;
                }
                return Ok(Some(Selected {
                    outputs: screens.into_iter().map(|screen| screen.frozen).collect(),
                    gesture,
                    size,
                    area,
                }));
            }

            self.redraw_selection(target, &mut screens)?;
            self.event_queue.blocking_dispatch(&mut self.state)?;
        }
    }

    fn redraw_selection(
        &mut self,
        target: SelectionTarget,
        screens: &mut [ScreenPreview],
    ) -> Result<(), Error> {
        for screen in screens {
            let output_id = screen.frozen.output.id;
            let overlay = self
                .state
                .overlays
                .get(&output_id)
                .ok_or(Error::OverlayClosed)?;
            let Some((size, area)) = self.state.overlay_area(output_id) else {
                continue;
            };
            let hovered = self
                .state
                .pointer_surface
                .as_ref()
                .is_some_and(|surface| overlay.is_surface(surface));
            let rectangle = target.rectangle(self.state.selection, size, area, hovered);
            let next = (size, rectangle);
            if screen.displayed == Some(next) || overlay.frame_callback.is_some() {
                continue;
            }
            self.display_preview(
                output_id,
                &screen.frozen.image,
                size,
                screen.transform,
                screen.frozen.orientation,
                rectangle,
            )?;
            screen.displayed = Some(next);
        }
        Ok(())
    }

    fn has_region(&self, gesture: Gesture, screens: &[ScreenPreview]) -> Result<bool, Error> {
        for screen in screens {
            let Some((size, area)) = self.state.overlay_area(screen.frozen.output.id) else {
                continue;
            };
            let Some(rectangle) = gesture.rectangle(size, area) else {
                continue;
            };
            if PixelRect::from_surface(
                rectangle,
                size,
                screen.frozen.image.size(),
                screen.frozen.orientation,
            )?
            .is_some()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn require_placements(outputs: &[FrozenOutput]) -> Result<(), Error> {
    for frozen in outputs {
        if frozen.output.logical.is_none() {
            return Err(Error::UnknownPlacement {
                name: frozen.output.name.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{DesktopPosition, SuggestedSurfaceSize, SurfacePosition};
    use crate::{Output, PhysicalSize};
    use claims::{assert_err, assert_none, assert_ok, assert_some};
    use tvert_image::{CapturedImage, ImageTransform, PixelSize};

    #[test]
    fn monitor_feedback_follows_hover_and_region_feedback_follows_drag() {
        let size = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 4,
            height: 2,
        }));
        let area = size.at_origin();
        let desktop = |x, y| {
            assert_some!(DesktopPosition::from_surface(
                assert_some!(SurfacePosition::new(x, y)),
                size,
                area
            ))
        };
        let mut selection = SelectionState::Idle;
        selection.begin(OutputId(1), desktop(0.0, 0.0));
        selection.update(desktop(2.0, 2.0));
        let screen = SelectionTarget::Screen;
        let region = SelectionTarget::Region(OutputId(1));
        assert_none!(screen.rectangle(selection, size, area, false));
        assert_eq!(
            screen.rectangle(selection, size, area, true),
            Some(SurfaceRect::full(size))
        );
        for region in [region, SelectionTarget::DesktopRegion] {
            for hovered in [false, true] {
                assert_eq!(
                    region.rectangle(selection, size, area, hovered),
                    selection.rectangle(size, area)
                );
                assert_none!(region.rectangle(SelectionState::Idle, size, area, hovered));
            }
        }
    }

    #[test]
    fn all_output_regions_require_every_placement() {
        let size = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 4,
            height: 2,
        }));
        let mut outputs = [1, 2]
            .into_iter()
            .map(|id| FrozenOutput {
                output: Output {
                    id: OutputId(id),
                    name: format!("DP-{id}"),
                    size: PhysicalSize {
                        width: 1,
                        height: 1,
                    },
                    scale: 1,
                    logical: Some(size.at_origin()),
                },
                image: assert_ok!(CapturedImage::new(
                    assert_ok!(PixelSize::try_from((1, 1))),
                    vec![10, 20, 30, 255],
                )),
                orientation: ImageTransform::Normal,
            })
            .collect::<Vec<_>>();
        assert_ok!(require_placements(&outputs));
        assert_some!(outputs.last_mut()).output.logical = None;
        assert!(matches!(
            assert_err!(require_placements(&outputs)),
            Error::UnknownPlacement { name } if name == "DP-2"
        ));
    }
}
