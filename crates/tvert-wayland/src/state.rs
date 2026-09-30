use crate::{
    Output, OutputId,
    global::Global,
    output::OutputState,
    wayland::{OverlayState, ScreencopyState},
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
    ext::{
        image_capture_source::v1::client::{
            ext_image_capture_source_v1::ExtImageCaptureSourceV1,
            ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        },
        image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    },
    wp::viewporter::client::wp_viewporter::WpViewporter,
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

    pub shm: Option<WlShm>,
    pub shm_formats: Vec<Format>,

    pub output_capture_source_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    pub capture_manager: Option<ExtImageCopyCaptureManagerV1>,

    pub screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    pub screencopy: ScreencopyState,

    pub compositor: Option<WlCompositor>,
    pub layer_shell: Option<ZwlrLayerShellV1>,
    pub overlay: Option<OverlayState>,
    pub viewporter: Option<WpViewporter>,
    pub capture_source: Option<(OutputId, ExtImageCaptureSourceV1)>,

    pub seat: Option<WlSeat>,
    pub pointer: Option<WlPointer>,
    pub pointer_surface: Option<WlSurface>,
}
