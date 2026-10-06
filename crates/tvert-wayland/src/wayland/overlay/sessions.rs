use crate::{
    Error, OutputId, Wayland,
    wayland::{
        overlay::{
            OverlayBuffer, SelectionState, SuggestedSurfaceSize, SurfaceRect, render,
            size::SurfaceSize,
            state::{OverlayState, OverlayStatus},
        },
        shm::ShmBuffer,
    },
};
use tvert_image::CapturedImage;
use wayland_client::{
    Proxy, WEnum,
    protocol::{wl_compositor::WlCompositor, wl_output::Transform, wl_shm::WlShm},
};
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity},
};

impl Wayland {
    /// Creates an overlay and waits for its initial configuration.
    ///
    /// Retains the surface without attaching a buffer or requesting keyboard focus. Replaces any
    /// previously retained overlay.
    ///
    /// Obtain `output_id` from this instance's [`Self::outputs`] method.
    ///
    /// # Errors
    ///
    /// Returns
    /// - [`Error::MissingGlobal`] if the compositor or layer-shell global is unavailable.
    /// - [`Error::UnknownOutput`] if the output is no longer tracked.
    /// - [`Error::OverlayClosed`] if the compositor closes the overlay.
    /// - [`Error::Dispatch`] if communicating with the compositor fails.
    pub fn prepare_overlay(&mut self, output_id: OutputId) -> Result<SuggestedSurfaceSize, Error> {
        let Some(compositor) = self.state.compositor.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: WlCompositor::interface().name,
            });
        };

        let Some(layer_shell) = self.state.layer_shell.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: ZwlrLayerShellV1::interface().name,
            });
        };

        let Some(viewporter) = self.state.viewporter.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: WpViewporter::interface().name,
            });
        };

        let Some(output) = self.state.output_states.get(&output_id) else {
            return Err(Error::UnknownOutput(output_id));
        };

        let queue_handle = self.event_queue.handle();
        let surface = compositor.create_surface(&queue_handle, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(&output.proxy),
            Layer::Overlay,
            String::from("tvert-selector"),
            &queue_handle,
            (),
        );
        let viewport = viewporter.get_viewport(&surface, &queue_handle, ());

        layer_surface.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, 0);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        self.state.overlay = Some(OverlayState {
            output_id,
            surface,
            layer_surface,
            viewport,
            status: OverlayStatus::Waiting,
            buffer: None,
            frame_callback: None,
            pointer_position: None,
            selection: SelectionState::default(),
        });

        if let Some(overlay) = self.state.overlay.as_ref() {
            overlay.surface.commit();
        }

        loop {
            let Some(overlay) = self.state.overlay.as_ref() else {
                return Err(Error::OverlayClosed);
            };

            match overlay.status {
                OverlayStatus::Waiting => {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                }
                OverlayStatus::Configured(configure) => {
                    return Ok(configure);
                }
                OverlayStatus::UnsupportedSize(size) => {
                    drop(self.state.overlay.take());

                    return Err(Error::UnsupportedOverlaySize {
                        width: size.width,
                        height: size.height,
                    });
                }
                OverlayStatus::Closed | OverlayStatus::Cancelled => {
                    drop(self.state.overlay.take());
                    return Err(Error::OverlayClosed);
                }
            }
        }
    }

    /// Displays a frozen capture until cancelled or closed by the compositor.
    ///
    /// Dragging with the left button displays a selection outline. Releasing
    /// keeps it visible; starting another drag replaces it.
    /// Redraws are paced by compositor frame callbacks.
    ///
    /// # Errors
    ///
    /// Returns errors from [`Self::capture_output`] or [`Self::prepare_overlay`].
    ///
    /// Returns [`Error::MissingGlobal`] if SHM is unavailable,
    /// [`Error::InvalidShmLayout`] or [`Error::UnsupportedShmFormat`] if the
    /// display buffer cannot be created, [`Error::ImageAllocation`] if image
    /// storage cannot be allocated, or [`Error::ShmIo`] if writing it fails.
    ///
    /// Returns [`Error::UnsupportedOverlaySize`] if the suggested surface
    /// dimensions cannot be used, [`Error::OverlayClosed`] if the overlay
    /// disappears before mapping, or [`Error::Dispatch`] if communication fails.
    ///
    /// Returns [`Error::UnsupportedOutputTransform`] if the output advertises
    /// an unknown transform.
    ///
    /// Returns [`Error::Image`] if constructing an outlined image fails.
    pub fn preview_output(&mut self, output_id: OutputId) -> Result<(), Error> {
        self.state.pointer_surface = None;
        drop(self.state.overlay.take());

        let result = self.run_preview(output_id);

        self.state.pointer_surface = None;
        drop(self.state.overlay.take());

        result
    }

    fn run_preview(&mut self, output_id: OutputId) -> Result<(), Error> {
        let image = self.capture_output(output_id)?;

        let Some(output) = self.state.output_states.get(&output_id) else {
            return Err(Error::UnknownOutput(output_id));
        };

        let transform = match output.transform {
            WEnum::Value(transform) => transform,
            WEnum::Unknown(value) => {
                return Err(Error::UnsupportedOutputTransform(value));
            }
        };

        let size = SurfaceSize::try_from(self.prepare_overlay(output_id)?)?;
        self.display_preview(&image, size, transform, None)?;
        let mut displayed = (size, None);

        loop {
            let Some(overlay) = self.state.overlay.as_ref() else {
                return Ok(());
            };

            let size = match overlay.status {
                OverlayStatus::Closed | OverlayStatus::Cancelled => return Ok(()),
                OverlayStatus::UnsupportedSize(size) => {
                    return Err(Error::UnsupportedOverlaySize {
                        width: size.width,
                        height: size.height,
                    });
                }
                OverlayStatus::Waiting => {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                    continue;
                }
                OverlayStatus::Configured(size) => SurfaceSize::try_from(size)?,
            };

            let rectangle = overlay.selection_rectangle();
            let next = (size, rectangle);

            if next != displayed {
                self.display_preview(&image, size, transform, rectangle)?;
                displayed = next;
            }

            self.event_queue.blocking_dispatch(&mut self.state)?;
        }
    }

    fn display_preview(
        &mut self,
        image: &CapturedImage,
        size: SurfaceSize,
        transform: Transform,
        rectangle: Option<SurfaceRect>,
    ) -> Result<(), Error> {
        let rendered = rectangle
            .map(|rectangle| render::selection_outline(image, rectangle, size, transform))
            .transpose()?;
        let display_image = rendered.as_ref().unwrap_or(image);

        let Some(shm) = self.state.shm.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: WlShm::interface().name,
            });
        };

        let queue_handle = self.event_queue.handle();
        let buffer =
            ShmBuffer::from_image(shm, &self.state.shm_formats, display_image, &queue_handle)?;

        let image_size = display_image.size();
        let width = i32::try_from(image_size.width().get()).map_err(|_| Error::InvalidShmLayout)?;
        let height =
            i32::try_from(image_size.height().get()).map_err(|_| Error::InvalidShmLayout)?;

        let Some(overlay) = self.state.overlay.as_mut() else {
            return Err(Error::OverlayClosed);
        };

        overlay.surface.set_buffer_transform(transform);
        overlay.set_destination(size);
        overlay.surface.attach(Some(buffer.proxy()), 0, 0);
        overlay.surface.damage_buffer(0, 0, width, height);
        overlay.frame_callback = Some(overlay.surface.frame(&queue_handle, ()));

        let previous = overlay.buffer.replace(OverlayBuffer::Submitted(buffer));
        overlay.surface.commit();
        drop(previous);

        Ok(())
    }
}
