mod capture;
mod dispatch;
mod input;
mod output;
mod overlay;
mod pixels;
mod registry;
mod screencopy;
mod shm;

pub use output::OutputState;
pub use overlay::OverlayState;
pub use screencopy::ScreencopyState;

use crate::{Error, Global, Output, state::State};
use tracing::{debug, trace};
use wayland_client::{Connection, EventQueue, globals::registry_queue_init};

/// Owns the Wayland event queue and protocol state
#[derive(Debug)]
pub struct Wayland {
    event_queue: EventQueue<State>,
    state: State,
}

impl Wayland {
    /// Connects to the compositor and discovers globals and initial output metadata.
    ///
    /// # Errors
    ///
    /// Returns
    /// - [`Error::Connect`] if the connection cannot be established;
    /// - [`Error::Dispatch`] if dispatching the initial events fails.
    pub fn connect() -> Result<Self, Error> {
        let connection = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init::<State>(&connection)?;
        let queue_handle = event_queue.handle();

        let mut state = State {
            globals: globals
                .contents()
                .clone_list()
                .into_iter()
                .map(|global| Global {
                    name: global.name,
                    interface: global.interface,
                    version: global.version,
                })
                .collect(),
            ..Default::default()
        };

        state.bind_advertised_globals(globals.registry(), &queue_handle);

        let mut wayland = Self { event_queue, state };

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
            xdg_output_manager_bound = wayland.state.xdg_output_manager.is_some(),
            seat_bound = wayland.state.seat.is_some(),
            pointer_created = wayland.state.pointer.is_some(),
            "capture global binding status"
        );

        for global in &wayland.state.globals {
            trace!(%global, "advertised global");
        }
        for output in wayland.state.outputs.values() {
            trace!(%output, "discovered output");
        }

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
