use std::{fmt::Display, num::NonZeroU32};

/// Identifies an output by its registry name on one [`Wayland`](crate::Wayland)
/// connection.
///
/// Obtain it from [`Wayland::outputs`](crate::Wayland::outputs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

/// An output's area in the compositor's logical desktop coordinates.
///
/// Positions may be negative. The size already accounts for the output's
/// scale and transform, so a rotated output reports its displayed shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalRect {
    pub x: i32,
    pub y: i32,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub id: OutputId,
    pub name: String,
    pub size: PhysicalSize,
    pub scale: i32,
    pub logical: Option<LogicalRect>,
}

impl Display for PhysicalSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl Display for LogicalRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}{:+}{:+}", self.width, self.height, self.x, self.y)
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} scale {}", self.name, self.size, self.scale)?;
        if let Some(logical) = self.logical {
            write!(f, " at {logical}")?;
        }
        Ok(())
    }
}
