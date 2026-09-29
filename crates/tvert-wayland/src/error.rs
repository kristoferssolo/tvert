use crate::OutputId;
use wayland_client::{
    ConnectError, DispatchError, WEnum, globals::GlobalError, protocol::wl_shm::Format,
};

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

    /// Creating, sizing, or reading the backing file failed
    #[error("failed to access the SHM backing file")]
    ShmIo(#[from] std::io::Error),

    /// The retained frame is not in the state required for the operation
    #[error("invalid screencopy frame state")]
    InvalidScreencopyState,

    /// Allocating storage for the captured image failed
    #[error("failed to allocate captured image storage")]
    ImageAllocation(#[from] std::collections::TryReserveError),

    /// The compositor returned flags this implementation cannot interpret
    #[error("unsupported screencopy flags: {0:#x}")]
    UnsupportedScreencopyFlags(u32),

    /// The compositor closed the overlay before preparation completed
    #[error("the compositor closed the selection overlay")]
    OverlayClosed,

    /// Initial registry discovery failed
    #[error("failed to initialize the Wayland registry")]
    RegistryInit(#[from] GlobalError),
}
