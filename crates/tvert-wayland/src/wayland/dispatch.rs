use crate::state::State;
use tracing::debug;
use wayland_client::{
    Connection, Dispatch, Proxy, WEnum, delegate_noop,
    protocol::{
        wl_compositor::WlCompositor,
        wl_shm::{Event as ShmEvent, WlShm},
    },
};
use wayland_protocols::{
    wp::viewporter::client::wp_viewporter::WpViewporter,
    xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1,
};
use wayland_protocols_wlr::{
    layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1,
    screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

impl Dispatch<WlShm, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &WlShm,
        event: <WlShm as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let ShmEvent::Format { format } = event {
            debug!(?format, "compositor advertised a wl_shm format");

            let WEnum::Value(format) = format else {
                return;
            };

            if !state.shm_formats.contains(&format) {
                state.shm_formats.push(format);
            }
        }
    }
}

delegate_noop!(State: ignore WlCompositor);
delegate_noop!(State: ignore WpViewporter);
delegate_noop!(State: ignore ZwlrLayerShellV1);
delegate_noop!(State: ignore ZwlrScreencopyManagerV1);
delegate_noop!(State: ignore ZxdgOutputManagerV1);
