use crate::{
    Output, OutputId,
    global::Global,
    wayland::{OutputState, OverlayState, ScreencopyState},
};
use std::collections::HashMap;
use wayland_client::protocol::{
    wl_compositor::WlCompositor,
    wl_pointer::WlPointer,
    wl_seat::WlSeat,
    wl_shm::{Format, WlShm},
    wl_surface::WlSurface,
};
use wayland_protocols::{
    wp::{
        cursor_shape::v1::client::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
        viewporter::client::wp_viewporter::WpViewporter,
    },
    xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1,
};
use wayland_protocols_wlr::{
    layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1,
    screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

/// Protocol objects and metadata updated by this connection's event queue
#[derive(Debug, Default)]
pub struct State {
    pub globals: Vec<Global>,
    pub bound_globals: HashMap<&'static str, u32>,

    pub output_states: HashMap<OutputId, OutputState>,
    pub outputs: HashMap<OutputId, Output>,
    pub xdg_output_manager: Option<ZxdgOutputManagerV1>,

    pub shm: Option<WlShm>,
    pub shm_formats: Vec<Format>,

    pub screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    pub screencopy: ScreencopyState,

    pub compositor: Option<WlCompositor>,
    pub layer_shell: Option<ZwlrLayerShellV1>,
    pub overlays: HashMap<OutputId, OverlayState>,
    pub viewporter: Option<WpViewporter>,

    pub seat: Option<WlSeat>,
    pub pointer: Option<WlPointer>,
    pub pointer_surface: Option<WlSurface>,
    pub cursor_shape_manager: Option<WpCursorShapeManagerV1>,
}
