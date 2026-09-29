mod output;

pub use crate::output::{Output, OutputId, PhysicalSize};
use crate::output::{OutputData, OutputState};
use std::{collections::HashMap, fmt::Display};
use tracing::debug;
use wayland_client::{
    ConnectError, Connection, Dispatch, DispatchError, EventQueue, Proxy, WEnum,
    protocol::{
        wl_output::{Event as OutputEvent, Mode, WlOutput},
        wl_registry::{Event, WlRegistry},
        wl_shm::{Event as ShmEvent, WlShm},
    },
};
use wayland_protocols::ext::{
    image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
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
    output_states: HashMap<OutputId, OutputState>,
    outputs: HashMap<OutputId, Output>,

    shm: Option<WlShm>,
    output_capture_source_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    capture_manager: Option<ExtImageCopyCaptureManagerV1>,
}

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
    /// Returns [`Error::Connect`] if the connection cannot be established.
    ///
    /// Returns [`Error::Dispatch`] if dispatching the initial events fails.
    pub fn connect() -> Result<Self, Error> {
        let connection = Connection::connect_to_env()?;
        let event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();

        let display = connection.display();
        let _registry = display.get_registry(&queue_handle, ());

        let mut wayland = Self {
            event_queue,
            state: State::default(),
        };

        wayland.event_queue.roundtrip(&mut wayland.state)?;

        wayland.event_queue.roundtrip(&mut wayland.state)?;
        debug!(
            shm_bound = wayland.state.shm.is_some(),
            output_capture_source_manager_bound =
                wayland.state.output_capture_source_manager.is_some(),
            capture_manager_bound = wayland.state.capture_manager.is_some(),
            "capture global binding status"
        );

        Ok(wayland)
    }

    #[must_use]
    pub fn globals(&self) -> &[Global] {
        &self.state.globals
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.state.outputs.values()
    }
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            Event::Global {
                name,
                interface,
                version,
            } => {
                state.globals.push(Global {
                    name,
                    interface: interface.clone(),
                    version,
                });

                if interface == WlOutput::interface().name {
                    let id = OutputId(name);
                    let proxy = registry.bind(name, version.min(4), qh, OutputData { id });
                    state.output_states.insert(id, OutputState::new(proxy));
                    return;
                }
                if interface == WlShm::interface().name {
                    if state.shm.is_some() {
                        return;
                    }
                    state.shm = Some(registry.bind(name, 1, qh, ()));
                    return;
                }

                if interface == ExtOutputImageCaptureSourceManagerV1::interface().name {
                    if state.output_capture_source_manager.is_some() {
                        return;
                    }
                    state.output_capture_source_manager = Some(registry.bind(name, 1, qh, ()));
                    return;
                }

                if interface == ExtImageCopyCaptureManagerV1::interface().name {
                    if state.capture_manager.is_some() {
                        return;
                    }
                    state.capture_manager = Some(registry.bind(name, 1, qh, ()));
                }
            }
            Event::GlobalRemove { name } => {
                let id = OutputId(name);

                state.globals.retain(|global| global.name != name);
                state.output_states.remove(&id);
                state.outputs.remove(&id);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, OutputData> for State {
    fn event(
        state: &mut Self,
        _output: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        data: &OutputData,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(output) = state.output_states.get_mut(&data.id) else {
            return;
        };

        match event {
            OutputEvent::Name { name } => {
                output.name = Some(name);
            }
            OutputEvent::Scale { factor } => {
                output.scale = factor;
            }
            OutputEvent::Mode {
                flags,
                width,
                height,
                ..
            } => {
                let WEnum::Value(flags) = flags else {
                    return;
                };

                if !flags.contains(Mode::Current) {
                    return;
                }

                let Ok(width) = u32::try_from(width) else {
                    return;
                };

                let Ok(height) = u32::try_from(height) else {
                    return;
                };

                output.size = Some(PhysicalSize { width, height });
            }
            OutputEvent::Done => {
                if let Some(snapshot) = output.snapshot(data.id) {
                    state.outputs.insert(data.id, snapshot);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlShm, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        event: <WlShm as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let ShmEvent::Format { format } = event {
            debug!(?format, "compositor advertised a wl_shm format");
        }
    }
}

wayland_client::delegate_noop!(
    State: ignore ExtOutputImageCaptureSourceManagerV1
);

wayland_client::delegate_noop!(
    State: ignore ExtImageCopyCaptureManagerV1
);

impl Display for Global {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} v{}", self.name, self.interface, self.version)
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} scale {}", self.name, self.size, self.scale)
    }
}
