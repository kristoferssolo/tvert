use crate::state::State;
use wayland_client::{
    Dispatch, WEnum,
    protocol::wl_keyboard::{Event, KeyState, WlKeyboard},
};

const KEY_ESCAPE: u32 = 1;

impl State {
    pub(super) fn release_keyboard(&mut self) {
        self.keyboard_surface = None;
        if let Some(keyboard) = self.keyboard.take() {
            keyboard.release();
        }
    }
}

impl Dispatch<WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        keyboard: &WlKeyboard,
        event: <WlKeyboard as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        if state.keyboard.as_ref() != Some(keyboard) {
            return;
        }

        match event {
            Event::Enter { surface, .. } => state.keyboard_surface = Some(surface),
            Event::Leave { surface, .. } => {
                if state.keyboard_surface.as_ref() == Some(&surface) {
                    state.keyboard_surface = None;
                }
            }
            Event::Key {
                key: KEY_ESCAPE,
                state: WEnum::Value(KeyState::Pressed),
                ..
            } => {
                let focused = state.keyboard_surface.as_ref().is_some_and(|surface| {
                    state
                        .overlays
                        .values()
                        .any(|overlay| overlay.is_surface(surface) && overlay.is_interactive())
                });
                if !focused {
                    return;
                }
                for overlay in state.overlays.values_mut() {
                    overlay.cancel();
                }
            }
            _ => {}
        }
    }
}
