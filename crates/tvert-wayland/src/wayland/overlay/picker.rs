use crate::{
    Error, Output, Wayland,
    wayland::{
        output::buffer_to_surface,
        overlay::{SelectionState, SurfaceRect, size::SurfaceSize, state::OverlayStatus},
    },
};
use tvert_image::{CapturedImage, ImageTransform};
use wayland_client::{WEnum, protocol::wl_output::Transform};

struct FrozenScreen {
    output: Output,
    image: CapturedImage,
    transform: Transform,
    orientation: ImageTransform,
    displayed: Option<(SurfaceSize, bool)>,
}

impl Wayland {
    pub(super) fn run_screen_selection(&mut self) -> Result<Option<CapturedImage>, Error> {
        let mut screens = self.freeze_screens()?;

        // Capture every output before any selector surfacce is mapped
        for screen in &screens {
            self.prepare_overlay(screen.output.id)?;
        }

        loop {
            self.validate_screens(&screens)?;

            for screen in &screens {
                let Some(overlay) = self.state.overlays.get(&screen.output.id) else {
                    return Ok(None);
                };
                match overlay.status {
                    OverlayStatus::Closed | OverlayStatus::Cancelled => return Ok(None),
                    OverlayStatus::UnsupportedSize(size) => {
                        return Err(Error::UnsupportedOverlaySize {
                            width: size.width,
                            height: size.height,
                        });
                    }
                    OverlayStatus::Waiting | OverlayStatus::Configured(_) => {}
                }
            }
            let selected = screens.iter().find_map(|screen| {
                let overlay = self.state.overlays.get(&screen.output.id)?;
                matches!(overlay.selection, SelectionState::Complete { .. })
                    .then_some(screen.output.id)
            });
            if let Some(output_id) = selected {
                let screen = screens
                    .into_iter()
                    .find(|screen| screen.output.id == output_id)
                    .ok_or(Error::UnknownOutput(output_id))?;
                return Ok(Some(screen.image.transformed(screen.orientation)?));
            }

            for screen in &mut screens {
                let output_id = screen.output.id;
                let overlay = self
                    .state
                    .overlays
                    .get(&output_id)
                    .ok_or(Error::OverlayClosed)?;
                let OverlayStatus::Configured(suggested) = overlay.status else {
                    continue;
                };
                let size = SurfaceSize::try_from(suggested)?;
                let hovered = self
                    .state
                    .pointer_surface
                    .as_ref()
                    .is_some_and(|surface| overlay.is_surface(surface));
                let next = (size, hovered);
                if screen.displayed == Some(next) || overlay.frame_callback.is_some() {
                    continue;
                }

                self.display_preview(
                    output_id,
                    &screen.image,
                    size,
                    screen.transform,
                    screen.orientation,
                    hovered.then(|| SurfaceRect::full(size)),
                )?;
                screen.displayed = Some(next);
            }

            self.event_queue.blocking_dispatch(&mut self.state)?;
        }
    }

    fn freeze_screens(&mut self) -> Result<Vec<FrozenScreen>, Error> {
        let mut outputs = self.outputs().cloned().collect::<Vec<_>>();
        outputs.sort_unstable_by_key(|output| output.id.0);
        if outputs.is_empty() {
            return Err(Error::NoOutputs);
        }

        let screens = outputs
            .into_iter()
            .map(|output| {
                let state = self
                    .state
                    .output_states
                    .get(&output.id)
                    .ok_or(Error::UnknownOutput(output.id))?;
                let transform = match state.transform {
                    WEnum::Value(transform) => transform,
                    WEnum::Unknown(value) => return Err(Error::UnsupportedOutputTransform(value)),
                };
                let orientation = buffer_to_surface(transform)?;
                let image = self.capture_buffer(output.id)?;
                Ok(FrozenScreen {
                    output,
                    image,
                    transform,
                    orientation,
                    displayed: None,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.validate_screens(&screens)?;
        Ok(screens)
    }

    fn validate_screens(&self, screens: &[FrozenScreen]) -> Result<(), Error> {
        let unchanged = self.state.outputs.len() == screens.len()
            && screens.iter().all(|screen| {
                self.state.outputs.get(&screen.output.id) == Some(&screen.output)
                    && self
                        .state
                        .output_states
                        .get(&screen.output.id)
                        .is_some_and(|output| output.transform == WEnum::Value(screen.transform))
            });
        if !unchanged {
            return Err(Error::OutputsChanged);
        }
        Ok(())
    }
}
