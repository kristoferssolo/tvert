use crate::{
    Error, FrozenOutput, Wayland,
    wayland::overlay::{SelectionState, SurfaceRect, size::SurfaceSize, state::OverlayStatus},
};
use tvert_image::CapturedImage;

struct ScreenPreview {
    frozen: FrozenOutput,
    displayed: Option<(SurfaceSize, bool)>,
}

impl Wayland {
    pub(super) fn run_screen_selection(&mut self) -> Result<Option<CapturedImage>, Error> {
        let mut screens = self
            .freeze_outputs()?
            .into_iter()
            .map(|frozen| ScreenPreview {
                frozen,
                displayed: None,
            })
            .collect::<Vec<_>>();

        // Capture every output before any selector surfacce is mapped
        for screen in &screens {
            self.prepare_selection_overlay(screen.frozen.output.id)?;
        }

        loop {
            self.validate_frozen_ouputs(screens.iter().map(|screen| &screen.frozen))?;

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
            let selected = screens.iter().find_map(|screen| {
                let overlay = self.state.overlays.get(&screen.frozen.output.id)?;
                matches!(overlay.selection, SelectionState::Complete { .. })
                    .then_some(screen.frozen.output.id)
            });
            if let Some(output_id) = selected {
                let screen = screens
                    .into_iter()
                    .find(|screen| screen.frozen.output.id == output_id)
                    .ok_or(Error::UnknownOutput(output_id))?;
                return Ok(Some(screen.frozen.into_parts()?.1));
            }

            for screen in &mut screens {
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
                let next = (size, hovered);
                if screen.displayed == Some(next) || overlay.frame_callback.is_some() {
                    continue;
                }

                let transform = self
                    .state
                    .output_states
                    .get(&output_id)
                    .ok_or(Error::UnknownOutput(output_id))?
                    .transform()?;
                self.display_preview(
                    output_id,
                    &screen.frozen.image,
                    size,
                    transform,
                    screen.frozen.orientation,
                    hovered.then(|| SurfaceRect::full(size)),
                )?;
                screen.displayed = Some(next);
            }

            self.event_queue.blocking_dispatch(&mut self.state)?;
        }
    }
}
