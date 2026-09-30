mod error;
mod output;
mod wayland;

pub use crate::output::{Output, OutputId, PhysicalSize};
pub use error::Error;
pub use wayland::Wayland;

use crate::{
    output::OutputState,
    wayland::{OverlayState, ScreencopyState},
};
use std::{collections::HashMap, fmt::Display};
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

/// A global advertised by the Wayland compositor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

#[derive(Debug, Default)]
struct State {
    globals: Vec<Global>,
    bound_globals: HashMap<&'static str, u32>,

    output_states: HashMap<OutputId, OutputState>,
    outputs: HashMap<OutputId, Output>,

    shm: Option<WlShm>,
    shm_formats: Vec<Format>,

    output_capture_source_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    capture_manager: Option<ExtImageCopyCaptureManagerV1>,

    screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    screencopy: ScreencopyState,

    compositor: Option<WlCompositor>,
    layer_shell: Option<ZwlrLayerShellV1>,
    overlay: Option<OverlayState>,
    viewporter: Option<WpViewporter>,
    capture_source: Option<(OutputId, ExtImageCaptureSourceV1)>,

    seat: Option<WlSeat>,
    pointer: Option<WlPointer>,
    pointer_surface: Option<WlSurface>,
}

impl Display for Global {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} v{}", self.name, self.interface, self.version)
    }
}
