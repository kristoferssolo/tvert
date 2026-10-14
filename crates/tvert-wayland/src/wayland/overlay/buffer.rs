use crate::{state::State, wayland::shm::ShmBuffer};
use tracing::trace;
use wayland_client::{Dispatch, protocol::wl_buffer::Event, protocol::wl_buffer::WlBuffer};

/// A retained preview buffer and its compositor access state
#[derive(Debug)]
pub enum OverlayBuffer {
    /// Submitted to the compositor; its storage must remain unchanged
    Submitted(ShmBuffer),
    /// Released by the compositor; its storage may be reused
    Released(ShmBuffer),
}

impl OverlayBuffer {
    pub const fn proxy(&self) -> &WlBuffer {
        match self {
            Self::Submitted(buffer) | Self::Released(buffer) => buffer.proxy(),
        }
    }
}

impl Dispatch<WlBuffer, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &WlBuffer,
        event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        if !matches!(event, Event::Release) {
            return;
        }

        let Some(overlay) = state.overlay.as_mut() else {
            return;
        };

        let Some(current) = overlay.buffer.as_ref() else {
            return;
        };

        if current.proxy() != proxy {
            return;
        }

        let Some(buffer) = overlay.buffer.take() else {
            return;
        };

        let buffer = match buffer {
            OverlayBuffer::Submitted(buffer) | OverlayBuffer::Released(buffer) => buffer,
        };

        overlay.buffer = Some(OverlayBuffer::Released(buffer));
        trace!("overlay display buffer released");
    }
}
