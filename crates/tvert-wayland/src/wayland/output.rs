use crate::{Output, OutputId, PhysicalSize, state::State};
use wayland_client::{
    Connection, Dispatch, Proxy, WEnum,
    protocol::wl_output::{Event as OutputEvent, Mode, Transform, WlOutput},
};

#[derive(Debug, Clone)]
pub struct OutputData {
    pub id: OutputId,
}

#[derive(Debug, Clone)]
pub struct OutputState {
    pub proxy: WlOutput,
    pub name: Option<String>,
    pub size: Option<PhysicalSize>,
    pub scale: i32,
    pub transform: WEnum<Transform>,
}

impl OutputState {
    pub const fn new(proxy: WlOutput) -> Self {
        Self {
            proxy,
            name: None,
            size: None,
            scale: 1,
            transform: WEnum::Value(Transform::Normal),
        }
    }

    pub fn snapshot(&self, id: OutputId) -> Option<Output> {
        Some(Output {
            id,
            name: self.name.clone()?,
            size: self.size?,
            scale: self.scale,
        })
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
            OutputEvent::Geometry { transform, .. } => {
                output.transform = transform;
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
