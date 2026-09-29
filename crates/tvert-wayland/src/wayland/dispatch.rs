use crate::{PhysicalSize, State, output::OutputData};
use tracing::debug;
use wayland_client::{
    Connection, Dispatch, Proxy, WEnum, delegate_noop,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{Event as OutputEvent, Mode, WlOutput},
        wl_shm::{Event as ShmEvent, WlShm},
    },
};
use wayland_protocols::{
    ext::{
        image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    },
    wp::viewporter::client::wp_viewporter::WpViewporter,
};
use wayland_protocols_wlr::{
    layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1,
    screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

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
delegate_noop!(State: ignore WlCompositor);
delegate_noop!(State: ignore WpViewporter);
delegate_noop!(State: ignore ZwlrLayerShellV1);
delegate_noop!(State: ignore ZwlrScreencopyManagerV1);
