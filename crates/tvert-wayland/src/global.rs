use std::fmt::Display;

/// A global advertised by the Wayland compositor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

impl Display for Global {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} v{}", self.name, self.interface, self.version)
    }
}
