mod capture;
mod dispatch;
mod overlay;
mod pixels;
mod screencopy;
mod shm;

pub use overlay::OverlayState;
pub use pixels::CapturedImage;
pub use screencopy::ScreencopyState;

use crate::{Error, Global, Output, State};
use tracing::debug;
use wayland_client::{Connection, EventQueue};
use wayland_protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1;

/// Owns the Wayland event queue and protocol state
#[derive(Debug)]
pub struct Wayland {
    event_queue: EventQueue<State>,
    state: State,
    capture_source: Option<ExtImageCaptureSourceV1>,
}

impl Wayland {
    /// Connects to the compositor and discovers globals and initial output metadata.
    ///
    /// # Errors
    /// Returns
    /// - [`Error::Connect`] if the connection cannot be established;
    /// - [`Error::Dispatch`] if dispatching the initial events fails.
    pub fn connect() -> Result<Self, Error> {
        let connection = Connection::connect_to_env()?;
        let event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();

        let display = connection.display();
        let _registry = display.get_registry(&queue_handle, ());

        let mut wayland = Self {
            event_queue,
            state: State::default(),
            capture_source: None,
        };

        wayland.event_queue.roundtrip(&mut wayland.state)?;

        wayland.event_queue.roundtrip(&mut wayland.state)?;
        debug!(
            shm_bound = wayland.state.shm.is_some(),
            output_capture_source_manager_bound =
                wayland.state.output_capture_source_manager.is_some(),
            capture_manager_bound = wayland.state.capture_manager.is_some(),
            screencopy_manager_bound = wayland.state.screencopy_manager.is_some(),
            compositor_bound = wayland.state.compositor.is_some(),
            layer_shell_bound = wayland.state.layer_shell.is_some(),
            viewporter_bound = wayland.state.viewporter.is_some(),
            "capture global binding status"
        );

        Ok(wayland)
    }

    /// Returns the globals discovered on this connection
    #[must_use]
    pub fn globals(&self) -> &[Global] {
        &self.state.globals
    }

    /// Returns the output snapshots collected on this connection
    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.state.outputs.values()
    }
}
