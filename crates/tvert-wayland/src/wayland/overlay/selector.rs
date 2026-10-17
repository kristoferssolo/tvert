use crate::{
    Error, FrozenOutput, LogicalRect, OutputId, Wayland,
    wayland::overlay::{
        SelectionState, SurfaceRect, crop, selection::Gesture, size::SurfaceSize,
        state::OverlayStatus,
    },
};
use tracing::debug;
use tvert_image::CapturedImage;
use wayland_client::protocol::wl_output::Transform;

#[derive(Debug, Clone, Copy)]
pub(super) enum SelectionTarget {
    Screen,
    Region(OutputId),
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
            Self::Region(_) => selection.rectangle(size, area),
        }
    }
}

impl Wayland {
    fn prepare_selection(&mut self, target: SelectionTarget) -> Result<Vec<ScreenPreview>, Error> {
        let outputs = match target {
            SelectionTarget::Screen => self.freeze_outputs()?,
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
    ) -> Result<Option<CapturedImage>, Error> {
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
                match target {
                    SelectionTarget::Screen => {
                        let origin = gesture.origin();
                        let selected = screens
                            .into_iter()
                            .find(|screen| screen.frozen.output.id == origin)
                            .ok_or(Error::UnknownOutput(origin))?;
                        return Ok(Some(selected.frozen.into_parts()?.1));
                    }
                    SelectionTarget::Region(output_id) => {
                        if let Some(image) = self.crop_region(output_id, gesture, &screens)? {
                            return Ok(Some(image));
                        }
                        debug!("selection has no area on the output");
                    }
                }
            }

            self.redraw_selection(target, &mut screens)?;
            self.event_queue.blocking_dispatch(&mut self.state)?;
        }
    }

    /// Crops a completed gesture from one output's frozen capture.
    ///
    /// Returns `None` if the gesture covers no pixels of that output.
    fn crop_region(
        &self,
        output_id: OutputId,
        gesture: Gesture,
        screens: &[ScreenPreview],
    ) -> Result<Option<CapturedImage>, Error> {
        let sceen = screens
            .iter()
            .find(|screen| screen.frozen.output.id == output_id)
            .ok_or(Error::UnknownOutput(output_id))?;
        let Some((size, area)) = self.state.overlay_area(output_id) else {
            return Ok(None);
        };
        let Some(rectangle) = gesture.rectangle(size, area) else {
            return Ok(None);
        };
        crop::selection(
            &sceen.frozen.image,
            rectangle,
            size,
            sceen.frozen.orientation,
        )
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{DesktopPosition, SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_none, assert_ok, assert_some};

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
        for hovered in [false, true] {
            assert_eq!(
                region.rectangle(selection, size, area, hovered),
                selection.rectangle(size, area)
            );
            assert_none!(region.rectangle(SelectionState::Idle, size, area, hovered));
        }
    }
}
