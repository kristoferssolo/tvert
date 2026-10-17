use crate::{
    OutputId,
    state::State,
    wayland::{DesktopPosition, SelectionState, overlay::SurfacePosition},
};
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
    /// Clears pointer focus, cancels an unfinished drag, and releases the pointer.
    ///
    /// Without a pointer, the drag's button release can never arrive.
    pub fn release_pointer(&mut self) {
        self.clear_pointer_focus();
        self.selection.cancel_drag();

        if let Some(pointer) = self.pointer.take() {
            pointer.release();
        }
    }

    /// Forgets focus and selection progress left by an earlier selector
    pub fn reset_selector_input(&mut self) {
        self.clear_pointer_focus();
        self.keyboard_surface = None;
        self.selection = SelectionState::Idle;
    }

    fn focused_output(&self) -> Option<OutputId> {
        let surface = self.pointer_surface.as_ref()?;
        self.overlays
            .iter()
            .find(|(_, overlay)| overlay.is_surface(surface))
            .map(|(output_id, _)| *output_id)
    }

    /// Forgets the focused surface without affecting the drag.
    ///
    /// Compositors that move focus mid-drag report later motion relative to
    /// the newly focused selector surface, which maps to the same desktop.
    fn clear_pointer_focus(&mut self) {
        self.pointer_surface = None;
        self.pointer_position = None;
    }

    fn update_pointer_position(&mut self, x: f64, y: f64) {
        let mapped = self.focused_output().and_then(|output_id| {
            let (size, area) = self.overlay_area(output_id)?;
            let position = DesktopPosition::from_surface(SurfacePosition::new(x, y)?, size, area)?;
            Some((output_id, position))
        });
        self.pointer_position = mapped.map(|(_, position)| position);

        let Some((output_id, position)) = mapped else {
            self.selection.cancel_drag();
            return;
        };

        self.selection.update(position);
        trace!(
            ?output_id,
            x = position.x(),
            y = position.y(),
            "desktop pointer position"
        );
    }

    fn handle_overlay_button(&mut self, button: u32, button_state: ButtonState) {
        let Some(output_id) = self.focused_output() else {
            return;
        };

        let Some(overlay) = self.overlays.get_mut(&output_id) else {
            return;
        };

        if !overlay.is_interactive() {
            return;
        }

        match (button, button_state) {
            (BUTTON_LEFT, ButtonState::Pressed) => {
                let Some(position) = self.pointer_position else {
                    return;
                };

                self.selection.begin(output_id, position);
                debug!(?position, "selection drag started");
            }
            (BUTTON_LEFT, ButtonState::Released) => {
                self.selection.finish();
                debug!(selection = ?self.selection, "selection drag released");
            }
            (BUTTON_RIGHT, ButtonState::Pressed) => overlay.cancel(),
            _ => {}
        }
    }

    fn set_overlay_cursor(&self, pointer: &WlPointer, serial: u32, qh: &QueueHandle<Self>) {
        if self.focused_output().is_none() {
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

        if capabilities.contains(Capability::Pointer) && state.pointer.is_none() {
            state.pointer = Some(seat.get_pointer(qh, ()));
        }
        if !capabilities.contains(Capability::Pointer) {
            state.release_pointer();
        }
        if capabilities.contains(Capability::Keyboard) && state.keyboard.is_none() {
            state.keyboard = Some(seat.get_keyboard(qh, ()));
        }
        if !capabilities.contains(Capability::Keyboard) {
            state.release_keyboard();
        }
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
                state.pointer_surface = Some(surface);
                state.update_pointer_position(surface_x, surface_y);
                state.set_overlay_cursor(pointer, serial, qh);
            }
            PointerEvent::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.update_pointer_position(surface_x, surface_y);
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
