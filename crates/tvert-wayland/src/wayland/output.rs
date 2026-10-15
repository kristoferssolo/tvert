use crate::{Error, LogicalRect, Output, OutputId, PhysicalSize, state::State};
use std::num::NonZeroU32;
use tvert_image::ImageTransform;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
    protocol::wl_output::{Event as OutputEvent, Mode, Transform, WlOutput},
};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{
    Event as XdgOutputEvent, ZxdgOutputV1,
};

#[derive(Debug, Clone)]
pub struct OutputData {
    pub id: OutputId,
}

#[derive(Debug, Clone)]
pub struct OutputState {
    pub proxy: WlOutput,
    pub xdg_output: Option<ZxdgOutputV1>,
    pub name: Option<String>,
    pub size: Option<PhysicalSize>,
    pub scale: i32,
    pub transform: WEnum<Transform>,
    pub logical_position: Option<(i32, i32)>,
    pub logical_size: Option<(NonZeroU32, NonZeroU32)>,
}

impl OutputState {
    pub const fn new(proxy: WlOutput) -> Self {
        Self {
            proxy,
            xdg_output: None,
            name: None,
            size: None,
            scale: 1,
            transform: WEnum::Value(Transform::Normal),
            logical_position: None,
            logical_size: None,
        }
    }

    fn snapshot(&self, id: OutputId) -> Option<Output> {
        Some(Output {
            id,
            name: self.name.clone()?,
            size: self.size?,
            scale: self.scale,
            logical: self.logical(),
        })
    }

    /// Returns the transform that orients a captured buffer as displayed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedOutputTransform`] for an unknown transform.
    pub fn displayed_transform(&self) -> Result<ImageTransform, Error> {
        buffer_to_surface(self.transform()?)
    }

    /// Returns the known output buffer transform.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedOutputTransform`] for an unknown transform.
    pub const fn transform(&self) -> Result<Transform, Error> {
        match self.transform {
            WEnum::Value(transform) => Ok(transform),
            WEnum::Unknown(value) => Err(Error::UnsupportedOutputTransform(value)),
        }
    }

    fn logical(&self) -> Option<LogicalRect> {
        let (x, y) = self.logical_position?;
        let (width, height) = self.logical_size?;
        Some(LogicalRect {
            x,
            y,
            width,
            height,
        })
    }
}

pub fn buffer_to_surface(transform: Transform) -> Result<ImageTransform, Error> {
    // set_buffer_transform describes an operation already applied by the client.
    // The compositor displays its inverse.
    match transform {
        Transform::Normal => Ok(ImageTransform::Normal),
        Transform::_90 => Ok(ImageTransform::Rotate270),
        Transform::_180 => Ok(ImageTransform::Rotate180),
        Transform::_270 => Ok(ImageTransform::Rotate90),
        Transform::Flipped => Ok(ImageTransform::Flipped),
        Transform::Flipped90 => Ok(ImageTransform::Flipped90),
        Transform::Flipped180 => Ok(ImageTransform::Flipped180),
        Transform::Flipped270 => Ok(ImageTransform::Flipped270),
        _ => Err(Error::UnsupportedOutputTransform(u32::from(transform))),
    }
}

impl State {
    pub fn request_xdg_outputs(&mut self, qh: &QueueHandle<Self>) {
        let Some(manager) = self.xdg_output_manager.as_ref() else {
            return;
        };

        for (&id, output) in &mut self.output_states {
            if output.xdg_output.is_none() {
                let xdg_output = manager.get_xdg_output(&output.proxy, qh, OutputData { id });
                output.xdg_output = Some(xdg_output);
            }
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

impl Dispatch<ZxdgOutputV1, OutputData> for State {
    fn event(
        state: &mut Self,
        _xdg_output: &ZxdgOutputV1,
        event: <ZxdgOutputV1 as Proxy>::Event,
        data: &OutputData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let Some(output) = state.output_states.get_mut(&data.id) else {
            return;
        };

        match event {
            XdgOutputEvent::LogicalPosition { x, y } => output.logical_position = Some((x, y)),
            XdgOutputEvent::LogicalSize { width, height } => {
                output.logical_size = nonzero(width).zip(nonzero(height));
            }
            XdgOutputEvent::Done => {
                if let Some(snapshot) = output.snapshot(data.id) {
                    state.outputs.insert(data.id, snapshot);
                }
            }
            _ => {}
        }
    }
}

fn nonzero(value: i32) -> Option<NonZeroU32> {
    u32::try_from(value).ok().and_then(NonZeroU32::new)
}
