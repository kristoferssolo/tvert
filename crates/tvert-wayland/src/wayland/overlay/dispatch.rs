use crate::{
    state::State,
    wayland::overlay::{SuggestedSurfaceSize, size::SurfaceSize, state::OverlayStatus},
};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Event, ZwlrLayerSurfaceV1,
};

use wayland_client::{
    Dispatch, Proxy, delegate_noop,
    protocol::{
        wl_callback::{Event as CallbackEvent, WlCallback},
        wl_surface::WlSurface,
    },
};

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

                if overlay.buffer.is_none() {
                    overlay.status = OverlayStatus::Configured(size);
                    return;
                }

                let Ok(destination) = SurfaceSize::try_from(size) else {
                    overlay.status = OverlayStatus::UnsupportedSize(size);
                    return;
                };

                overlay.set_destination(destination);
                overlay.surface.commit();
                overlay.status = OverlayStatus::Configured(size);
            }
            Event::Closed => {
                overlay.status = OverlayStatus::Closed;
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        callback: &WlCallback,
        event: <WlCallback as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        if !matches!(event, CallbackEvent::Done { .. }) {
            return;
        }

        let Some(overlay) = state.overlay.as_mut() else {
            return;
        };

        if overlay.frame_callback.as_ref() != Some(callback) {
            return;
        }

        drop(overlay.frame_callback.take());
    }
}

delegate_noop!(State: ignore WlSurface);
delegate_noop!(State: ignore WpViewport);
