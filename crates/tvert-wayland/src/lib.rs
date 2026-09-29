mod output;
mod wayland;

pub use crate::output::{Output, OutputId, PhysicalSize};
pub use wayland::Wayland;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::output::OutputState;
use std::{collections::HashMap, fmt::Display};
use wayland_client::{ConnectError, DispatchError, protocol::wl_shm::WlShm};
use wayland_protocols::ext::{
    image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
};

/// A global advertised by the Wayland compositor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

/// An error connecting to or communicating with the Wayland compositor
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to connect to the Wayland compositor")]
    Connect(#[from] ConnectError),

    #[error("failed to dispatch Wayland events")]
    Dispatch(#[from] DispatchError),

    /// A required protocol global was not discovered
    #[error("required Wayland global is unavailable: {interface}")]
    MissingGlobal { interface: &'static str },

    /// The output is not present in this connection's tracked outputs
    #[error("unknown Wayland output: {0:?}")]
    UnknownOutput(OutputId),
}

#[derive(Debug, Default)]
struct State {
    globals: Vec<Global>,
    output_states: HashMap<OutputId, OutputState>,
    outputs: HashMap<OutputId, Output>,

    shm: Option<WlShm>,
    output_capture_source_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    capture_manager: Option<ExtImageCopyCaptureManagerV1>,
    screencopy_manager: Option<ZwlrScreencopyManagerV1>,
}

impl Display for Global {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} v{}", self.name, self.interface, self.version)
    }
}
