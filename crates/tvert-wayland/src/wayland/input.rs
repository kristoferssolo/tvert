use crate::state::State;
use tracing::trace;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
    protocol::{
        wl_pointer::{ButtonState, Event as PointerEvent, WlPointer},
        wl_seat::{Capability, Event as SeatEvent, WlSeat},
    },
};

// BTN_RIGHT from Linux input-event-codes.h
const BUTTON_RIGHT: u32 = 0x111;

impl State {
    /// Clears pointer focus and releases the retained pointer object
    pub fn release_pointer(&mut self) {
        self.pointer_surface = None;

        if let Some(pointer) = self.pointer.take() {
            pointer.release();
        }
    }

    fn clear_pointer_focus(&mut self) {
        self.pointer_surface = None;

        if let Some(overlay) = self.overlay.as_mut() {
            overlay.pointer_position = None;
        }
    }

    fn update_overlay_pointer_position(&mut self, x: f64, y: f64) {
        let Some(surface) = self.pointer_surface.as_ref() else {
            return;
        };

        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };

        if !overlay.is_surface(surface) {
            return;
        }

        overlay.pointer_position = Some((x, y));
        trace!(x, y, "overlay pointer position");
    }
}

impl Dispatch<WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        seat: &WlSeat,
        event: <WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        if state.seat.as_ref() != Some(seat) {
            return;
        }

        let SeatEvent::Capabilities { capabilities } = event else {
            return;
        };

        let capabilities = match capabilities {
            WEnum::Value(capabilities) => capabilities,
            WEnum::Unknown(bits) => Capability::from_bits_truncate(bits),
        };

        if !capabilities.contains(Capability::Pointer) {
            state.release_pointer();
            return;
        }

        if state.pointer.is_some() {
            return;
        }

        state.pointer = Some(seat.get_pointer(qh, ()));
    }
}

impl Dispatch<WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        pointer: &WlPointer,
        event: <WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if state.pointer.as_ref() != Some(pointer) {
            return;
        }

        match event {
            PointerEvent::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                state.clear_pointer_focus();
                pointer.set_cursor(serial, None, 0, 0);
                state.pointer_surface = Some(surface);
                state.update_overlay_pointer_position(surface_x, surface_y);
            }
            PointerEvent::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.update_overlay_pointer_position(surface_x, surface_y);
            }
            PointerEvent::Leave { surface, .. } => {
                if state.pointer_surface.as_ref() != Some(&surface) {
                    return;
                }
                state.clear_pointer_focus();
            }
            PointerEvent::Button {
                button: BUTTON_RIGHT,
                state: WEnum::Value(ButtonState::Pressed),
                ..
            } => {
                let Some(surface) = state.pointer_surface.as_ref() else {
                    return;
                };

                let Some(overlay) = state.overlay.as_mut() else {
                    return;
                };

                if !overlay.is_surface(surface) {
                    return;
                }

                overlay.cancel();
            }
            _ => {}
        }
    }
}
