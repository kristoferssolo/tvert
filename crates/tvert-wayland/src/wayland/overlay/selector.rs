use crate::{
    Error, FrozenOutput, OutputId, Wayland,
    wayland::overlay::{
        SelectionState, SurfaceRect, crop, size::SurfaceSize, state::OverlayStatus,
    },
};
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
        hovered: bool,
    ) -> Option<SurfaceRect> {
        match self {
            Self::Screen => hovered.then(|| SurfaceRect::full(size)),
            Self::Region(_) => selection.rectangle(size),
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

            for screen in &screens {
                let overlay = self
                    .state
                    .overlays
                    .get(&screen.frozen.output.id)
                    .ok_or(Error::OverlayClosed)?;
                let Some(size) = overlay.status.size()? else {
                    continue;
                };
                if !matches!(overlay.selection, SelectionState::Complete { .. }) {
                    continue;
                }
                match target {
                    SelectionTarget::Screen => {
                        let output_id = screen.frozen.output.id;
                        let selected = screens
                            .into_iter()
                            .find(|screen| screen.frozen.output.id == output_id)
                            .ok_or(Error::UnknownOutput(output_id))?;
                        return Ok(Some(selected.frozen.into_parts()?.1));
                    }
                    SelectionTarget::Region(_) => {
                        if let Some(rectangle) = overlay.selection_rectangle()
                            && let Some(image) = crop::selection(
                                &screen.frozen.image,
                                rectangle,
                                size,
                                screen.frozen.orientation,
                            )?
                        {
                            return Ok(Some(image));
                        }
                    }
                }
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
            let Some(size) = overlay.status.size()? else {
                continue;
            };
            let hovered = self
                .state
                .pointer_surface
                .as_ref()
                .is_some_and(|surface| overlay.is_surface(surface));
            let rectangle = target.rectangle(overlay.selection, size, hovered);
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
    use crate::wayland::overlay::{SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_none, assert_ok, assert_some};

    #[test]
    fn monitor_feedback_follows_hover_and_region_feedback_follows_drag() {
        let size = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 4,
            height: 2,
        }));
        let mut selection = SelectionState::Idle;
        selection.begin(assert_some!(SurfacePosition::new(0.0, 0.0)));
        selection.update(assert_some!(SurfacePosition::new(2.0, 2.0)));
        let screen = SelectionTarget::Screen;
        let region = SelectionTarget::Region(OutputId(1));
        assert_none!(screen.rectangle(selection, size, false));
        assert_eq!(
            screen.rectangle(selection, size, true),
            Some(SurfaceRect::full(size))
        );
        for hovered in [false, true] {
            assert_eq!(
                region.rectangle(selection, size, hovered),
                selection.rectangle(size)
            );
            assert_none!(region.rectangle(SelectionState::Idle, size, hovered));
        }
    }
}
