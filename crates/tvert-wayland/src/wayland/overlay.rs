use std::fmt::Display;

use crate::{Error, OutputId, State, Wayland};
use wayland_client::{
    Dispatch, Proxy, delegate_noop,
    protocol::{wl_compositor::WlCompositor, wl_surface::WlSurface},
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
    Closed,
}

#[derive(Debug)]
pub struct OverlayState {
    surface: WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    viewport: WpViewport,
    status: OverlayStatus,
}

impl Drop for OverlayState {
    fn drop(&mut self) {
        self.viewport.destroy();
        self.layer_surface.destroy();
        self.surface.destroy();
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
            surface,
            layer_surface,
            viewport,
            status: OverlayStatus::Waiting,
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
                OverlayStatus::Closed => {
                    drop(self.state.overlay.take());
                    return Err(Error::OverlayClosed);
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
                overlay.status = OverlayStatus::Configured(SuggestedSurfaceSize { width, height });
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
