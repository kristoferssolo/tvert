use std::fmt::Display;
use wayland_client::{
    ConnectError, Connection, Dispatch, DispatchError,
    protocol::wl_registry::{Event, WlRegistry},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to connect to the Wayland compositor")]
    Connect(#[from] ConnectError),

    #[error("failed to dispatch Wayland events")]
    Dispatch(#[from] DispatchError),
}

#[derive(Debug, Default)]
struct State {
    globals: Vec<Global>,
}

/// Returns the globals currently advertised by the Wayland compositor.
///
/// # Errors
///
/// Returns [`Error::Connect`] if a connection to the Wayland compositor cannot
/// be established.
///
/// Returns [`Error::Dispatch`] if dispatching the initial Wayland events fails.
pub fn globals() -> Result<Vec<Global>, Error> {
    let connection = Connection::connect_to_env()?;
    let mut event_queue = connection.new_event_queue();
    let queue_handle = event_queue.handle();

    let display = connection.display();
    let _registry = display.get_registry(&queue_handle, ());

    let mut state = State::default();

    event_queue.roundtrip(&mut state)?;

    Ok(state.globals)
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            Event::Global {
                name,
                interface,
                version,
            } => {
                state.globals.push(Global {
                    name,
                    interface,
                    version,
                });
            }
            Event::GlobalRemove { name } => {
                state.globals.retain(|global| global.name != name);
            }
            _ => {}
        }
    }
}

impl Display for Global {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} v{}", self.name, self.interface, self.version)
    }
}
