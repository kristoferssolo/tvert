use crate::{
    Error, OutputId, Wayland,
    wayland::{
        overlay::{
            OverlayBuffer, SelectionState, SuggestedSurfaceSize, SurfaceRect, render,
            selector::SelectionTarget,
            size::SurfaceSize,
            state::{OverlayState, OverlayStatus},
        },
        shm::ShmBuffer,
    },
};
use tvert_image::{CapturedImage, ImageTransform};
use wayland_client::{
    Proxy,
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
    /// overlay already retained for the same output. Returns the raw suggested
    /// dimensions, including zero dimensions the client would need to choose.
    ///
    /// # Errors
    ///
    /// Returns
    /// - [`Error::MissingGlobal`] if the compositor, layer-shell, or viewporter global is unavailable.
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

        let previous = self.state.overlays.insert(
            output_id,
            OverlayState {
                surface,
                layer_surface,
                viewport,
                status: OverlayStatus::Waiting,
                buffer: None,
                frame_callback: None,
                pointer_position: None,
                selection: SelectionState::default(),
            },
        );
        drop(previous);

        if let Some(overlay) = self.state.overlays.get(&output_id) {
            overlay.surface.commit();
        }

        loop {
            let Some(overlay) = self.state.overlays.get(&output_id) else {
                return Err(Error::OverlayClosed);
            };

            match overlay.status {
                OverlayStatus::Waiting => {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                }
                OverlayStatus::Configured(configure) => {
                    return Ok(configure.into());
                }
                OverlayStatus::UnsupportedSize(size) => {
                    drop(self.state.overlays.remove(&output_id));

                    return Err(Error::UnsupportedOverlaySize {
                        width: size.width,
                        height: size.height,
                    });
                }
                OverlayStatus::Closed | OverlayStatus::Cancelled => {
                    drop(self.state.overlays.remove(&output_id));
                    return Err(Error::OverlayClosed);
                }
            }
        }
    }

    /// Captures an output and waits for a nonempty region selection.
    ///
    /// Dragging with the left button displays a selection outline. Releasing
    /// returns an owned image oriented as displayed. Empty drags keep waiting.
    /// Escape, right-clicking, or a compositor closure after mapping returns `None`.
    ///
    /// The overlay is destroyed before this method returns.
    ///
    /// # Errors
    ///
    /// Returns any error from [`Self::capture_output`] or [`Self::prepare_overlay`].
    /// Returns [`Error::UnsupportedOverlaySize`] if the suggested size cannot be used.
    /// Displaying the selection can also return [`Error::MissingGlobal`] if SHM is
    /// unavailable, or [`Error::InvalidShmLayout`], [`Error::UnsupportedShmFormat`],
    /// [`Error::ImageAllocation`], [`Error::ShmIo`], or [`Error::Image`] if the
    /// outline or display buffer cannot be created. [`Error::Dispatch`] also
    /// covers overlay cleanup.
    pub fn select_region(&mut self, output_id: OutputId) -> Result<Option<CapturedImage>, Error> {
        self.select(SelectionTarget::Region(output_id))
    }

    /// Captures all outputs, then waits for a monitor to be clicked.
    ///
    /// The hovered monitor is shown at normal brightness with a dashed outline.
    /// Releasing the left button returns its original capture, oriented as
    /// displayed. Escape, right-clicking, or closure after mapping returns `None`.
    /// All selector surfaces are destroyed before returning, including on error.
    ///
    /// # Errors
    ///
    /// Returns any error from [`Self::select_region`], [`Error::NoOutputs`] if
    /// no outputs exist, or [`Error::OutputsChanged`] if output metadata changes
    /// while preparing or displaying the selector.
    pub fn select_screen(&mut self) -> Result<Option<CapturedImage>, Error> {
        self.select(SelectionTarget::Screen)
    }

    pub(super) fn prepare_selection_overlay(
        &mut self,
        output_id: OutputId,
    ) -> Result<SurfaceSize, Error> {
        self.prepare_overlay(output_id)?;
        self.state
            .overlays
            .get(&output_id)
            .ok_or(Error::OverlayClosed)?
            .status
            .size()?
            .ok_or(Error::OverlayClosed)
    }

    fn select(&mut self, target: SelectionTarget) -> Result<Option<CapturedImage>, Error> {
        self.state.pointer_surface = None;
        self.state.keyboard_surface = None;
        self.state.overlays.clear();

        let result = self.run_selection(target);
        self.state.overlays.clear();

        // Send destruction requests before handing the image to another stage.
        // Preserve the original error if both selection and cleanup fail.
        let cleanup = self.event_queue.roundtrip(&mut self.state);
        self.state.pointer_surface = None;
        self.state.keyboard_surface = None;

        let selected = result?;
        cleanup?;

        Ok(selected)
    }

    pub(super) fn display_preview(
        &mut self,
        output_id: OutputId,
        image: &CapturedImage,
        size: SurfaceSize,
        transform: Transform,
        orientation: ImageTransform,
        rectangle: Option<SurfaceRect>,
    ) -> Result<(), Error> {
        let image = render::selection_preview(image, rectangle, size, orientation)?;

        let Some(shm) = self.state.shm.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: WlShm::interface().name,
            });
        };

        let queue_handle = self.event_queue.handle();
        let buffer = ShmBuffer::from_image(shm, &self.state.shm_formats, &image, &queue_handle)?;

        let (width, height) = image.size().try_into()?;

        let Some(overlay) = self.state.overlays.get_mut(&output_id) else {
            return Err(Error::OverlayClosed);
        };

        if overlay.buffer.is_none() {
            overlay
                .layer_surface
                .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        }
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
