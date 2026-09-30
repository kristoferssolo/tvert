use crate::{Error, OutputId, State, Wayland, wayland::shm::ShmBuffer};
use std::fmt::Display;
use wayland_client::{
    Dispatch, Proxy, delegate_noop,
    protocol::{wl_compositor::WlCompositor, wl_shm::WlShm, wl_surface::WlSurface},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, Event, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

/// Suggested dimensions in surface-local coordinates.
///
/// A zero dimension means the client must choose that dimension.
#[derive(Debug, Clone, Copy)]
pub struct SuggestedSurfaceSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
enum OverlayStatus {
    Waiting,
    Configured(SuggestedSurfaceSize),
    UnsupoortedSize(SuggestedSurfaceSize),
    Closed,
}

#[derive(Debug)]
pub struct OverlayState {
    output_id: OutputId,
    surface: WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    viewport: WpViewport,
    status: OverlayStatus,
    buffer: Option<ShmBuffer>,
}

impl OverlayState {
    pub const fn output_id(&self) -> OutputId {
        self.output_id
    }

    fn set_destination(&self, size: SuggestedSurfaceSize) -> Result<(), Error> {
        let unsupported = || Error::UnsupportedOverlaySize {
            width: size.width,
            height: size.height,
        };

        if size.width == 0 || size.height == 0 {
            return Err(unsupported());
        }

        let width = i32::try_from(size.width).map_err(|_| unsupported())?;
        let height = i32::try_from(size.height).map_err(|_| unsupported())?;

        self.viewport.set_destination(width, height);

        Ok(())
    }
}

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
    /// Returns [`Error::MissingGlobal`] if the compositor or layer-shell global is unavailable.
    ///
    /// Returns [`Error::UnknownOutput`] if the output is no longer tracked.
    ///
    /// Returns [`Error::OverlayClosed`] if the compositor closes the overlay.
    ///
    /// Returns [`Error::Dispatch`] if communicating with the compositor fails.
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
                OverlayStatus::UnsupoortedSize(size) => {
                    drop(self.state.overlay.take());

                    return Err(Error::UnsupportedOverlaySize {
                        width: size.width,
                        height: size.height,
                    });
                }
                OverlayStatus::Closed => {
                    drop(self.state.overlay.take());
                    return Err(Error::OverlayClosed);
                }
            }
        }
    }

    /// Displays a frozen capture until the compositor closes the overlay.
    ///
    /// Captures before creating the overlay and leaves keyboard focus unchanged.
    /// Replaces any existing overlay.
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
    pub fn preview_output(&mut self, output_id: OutputId) -> Result<(), Error> {
        drop(self.state.overlay.take());

        let result = self.run_preview(output_id);

        drop(self.state.overlay.take());
        result
    }

    fn run_preview(&mut self, output_id: OutputId) -> Result<(), Error> {
        let image = self.capture_output(output_id)?;
        let size = self.prepare_overlay(output_id)?;

        let Some(shm) = self.state.shm.as_ref() else {
            return Err(Error::MissingGlobal {
                interface: WlShm::interface().name,
            });
        };

        let queue_handle = self.event_queue.handle();
        let buffer = ShmBuffer::from_image(shm, &self.state.shm_formats, &image, &queue_handle)?;

        let width = i32::try_from(image.width.get()).map_err(|_| Error::InvalidShmLayout)?;
        let height = i32::try_from(image.height.get()).map_err(|_| Error::InvalidShmLayout)?;

        let Some(overlay) = self.state.overlay.as_mut() else {
            return Err(Error::OverlayClosed);
        };

        overlay.set_destination(size)?;
        overlay.surface.attach(Some(buffer.proxy()), 0, 0);
        overlay.surface.damage_buffer(0, 0, width, height);
        overlay.buffer = Some(buffer);
        overlay.surface.commit();

        loop {
            let Some(overlay) = self.state.overlay.as_ref() else {
                return Ok(());
            };

            match overlay.status {
                OverlayStatus::Closed => return Ok(()),
                OverlayStatus::UnsupoortedSize(size) => {
                    return Err(Error::UnsupportedOverlaySize {
                        width: size.width,
                        height: size.height,
                    });
                }
                OverlayStatus::Waiting | OverlayStatus::Configured(_) => {
                    self.event_queue.blocking_dispatch(&mut self.state)?;
                }
            }
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(overlay) = state.overlay.as_mut() else {
            return;
        };

        if &overlay.layer_surface != layer_surface {
            return;
        }

        match event {
            Event::Configure {
                serial,
                width,
                height,
            } => {
                layer_surface.ack_configure(serial);

                let size = SuggestedSurfaceSize { width, height };

                if overlay.buffer.is_some() {
                    if overlay.set_destination(size).is_err() {
                        overlay.status = OverlayStatus::UnsupoortedSize(size);
                        return;
                    }
                    overlay.surface.commit();
                }

                overlay.status = OverlayStatus::Configured(size);
            }
            Event::Closed => {
                overlay.status = OverlayStatus::Closed;
            }
            _ => {}
        }
    }
}

delegate_noop!(State: ignore WlSurface);
delegate_noop!(State: ignore WpViewport);

impl Display for SuggestedSurfaceSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl Drop for OverlayState {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
