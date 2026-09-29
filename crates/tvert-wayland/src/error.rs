use crate::OutputId;
use wayland_client::{ConnectError, DispatchError, WEnum, protocol::wl_shm::Format};

/// An error connecting to or communicating with the Wayland compositor
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to connect to the Wayland compositor")]
    Connect(#[from] ConnectError),

    #[error("failed to dispatch Wayland events")]
    Dispatch(#[from] DispatchError),

    /// A required protocol global was not discovered
    #[error("required Wayland global is unavailable: {interface}")]
    MissingGlobal { interface: &'static str },

    /// The output is not present in this connection's tracked outputs
    #[error("unknown Wayland output: {0:?}")]
    UnknownOutput(OutputId),

    /// The compositor rejected or aborted the screencopy frame
    #[error("the compositor failed to capture the frame")]
    ScreencopyFailed,

    /// The frame does not advertise a shared-memory buffer
    #[error("the screencopy frame does not support SHM buffers")]
    ScreencopyShmUnavailable,

    /// The advertised SHM layout cannot be represented or is invalid
    #[error("invalid or oversized SHM buffer layout")]
    InvalidShmLayout,

    /// The format is unsupported by Tvert or unavailable through `wl_shm`
    #[error("unsupported SHM pixel format: {0:?}")]
    UnsupportedShmFormat(WEnum<Format>),

    /// Creating or sizing the backing file failed
    #[error("failed to allocate the SHM backing file")]
    ShmIo(#[from] std::io::Error),

    /// The retained frame is not in the state required for the operation
    #[error("invalid screencopy frame state")]
    InvalidScreencopyState,
}
