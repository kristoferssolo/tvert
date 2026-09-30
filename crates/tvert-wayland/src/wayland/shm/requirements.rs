use wayland_client::{WEnum, protocol::wl_shm::Format};

/// SHM buffer requirements advertised for a screencopy frame
#[derive(Debug, Clone, Copy)]
pub struct ShmRequirements {
    /// Pixel format, including formats unknown to this client
    pub format: WEnum<Format>,
    pub width: u32,
    pub height: u32,
    /// Number of bytes between consecutive rows
    pub stride: u32,
}
