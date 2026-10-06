use crate::{state::State, wayland::overlay::SurfacePosition};
use tracing::{debug, trace};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop,
    protocol::{
        wl_pointer::{ButtonState, Event as PointerEvent, WlPointer},
        wl_seat::{Capability, Event as SeatEvent, WlSeat},
    },
};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1::{Shape, WpCursorShapeDeviceV1},
    wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
};

// BTN_RIGHT from Linux input-event-codes.h
const BUTTON_RIGHT: u32 = 0x111;
// BTN_LEFT from Linux input-event-codes.h
const BUTTON_LEFT: u32 = 0x110;

impl State {
    /// Clears pointer focus and releases the retained pointer object
    pub fn release_pointer(&mut self) {
        self.clear_pointer_focus();

        if let Some(pointer) = self.pointer.take() {
            pointer.release();
        }
    }

    fn clear_pointer_focus(&mut self) {
        self.pointer_surface = None;

        if let Some(overlay) = self.overlay.as_mut() {
            overlay.pointer_position = None;
            overlay.selection.cacel_drag();
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

        let Some(position) = SurfacePosition::new(x, y) else {
            overlay.pointer_position = None;
            overlay.selection.cacel_drag();
            return;
        };

        overlay.pointer_position = Some(position);
        overlay.selection.update(position);
        trace!(
            x = position.x(),
            y = position.y(),
            "overlay pointer position"
        );
    }

    fn handle_overlay_button(&mut self, button: u32, button_state: ButtonState) {
        let Some(surface) = self.pointer_surface.as_ref() else {
            return;
        };

        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };

        if !overlay.is_surface(surface) || !overlay.is_interactive() {
            return;
        }

        match (button, button_state) {
            (BUTTON_LEFT, ButtonState::Pressed) => {
                let Some(position) = overlay.pointer_position else {
                    return;
                };

                overlay.selection.begin(position);
                debug!(?position, "selection drag started");
            }
            (BUTTON_LEFT, ButtonState::Released) => {
                overlay.selection.finish();

                let Some(rectangle) = overlay.selection_rectangle() else {
                    debug!("selection drag has no area");
                    return;
                };

                debug!(
                    left = rectangle.left(),
                    top = rectangle.top(),
                    right = rectangle.right(),
                    bottom = rectangle.bottom(),
                    "selection drag complete"
                );
            }
            (BUTTON_RIGHT, ButtonState::Pressed) => overlay.cancel(),
            _ => {}
        }
    }

    fn set_overlay_crusor(&self, pointer: &WlPointer, serial: u32, qh: &QueueHandle<Self>) {
        let Some(surface) = self.pointer_surface.as_ref() else {
            return;
        };

        let Some(overlay) = self.overlay.as_ref() else {
            return;
        };

        if !overlay.is_surface(surface) {
            return;
        }

        let Some(manager) = self.cursor_shape_manager.as_ref() else {
            return;
        };

        let cursor = manager.get_pointer(pointer, qh, ());
        cursor.set_shape(serial, Shape::Crosshair);
        cursor.destroy();
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
        qh: &QueueHandle<Self>,
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
                state.pointer_surface = Some(surface);
                state.update_overlay_pointer_position(surface_x, surface_y);
                state.set_overlay_crusor(pointer, serial, qh);
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
                button,
                state: WEnum::Value(button_state),
                ..
            } => {
                state.handle_overlay_button(button, button_state);
            }
            _ => {}
        }
    }
}

delegate_noop!(State: ignore WpCursorShapeManagerV1);
delegate_noop!(State: ignore WpCursorShapeDeviceV1);
