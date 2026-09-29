use crate::{
    Global, OutputId, PhysicalSize, State,
    output::{OutputData, OutputState},
};
use tracing::debug;
use wayland_client::{
    Connection, Dispatch, Proxy, WEnum, delegate_noop,
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
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

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
                    return;
                }

                if interface == ZwlrScreencopyManagerV1::interface().name {
                    if state.screencopy_manager.is_some() {
                        return;
                    }
                    state.screencopy_manager = Some(registry.bind(name, version.min(3), qh, ()));
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

delegate_noop!(State: ignore ExtImageCopyCaptureManagerV1);
delegate_noop!(State: ignore ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(State: ignore ZwlrScreencopyManagerV1);
