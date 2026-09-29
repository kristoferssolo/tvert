use std::fmt::Display;
use wayland_client::protocol::wl_output::WlOutput;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub id: OutputId,
    pub name: String,
    pub size: PhysicalSize,
    pub scale: i32,
}

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
}

impl OutputState {
    pub const fn new(proxy: WlOutput) -> Self {
        Self {
            proxy,
            name: None,
            size: None,
            scale: 1,
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

impl Display for PhysicalSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} scale {}", self.name, self.size, self.scale)
    }
}
