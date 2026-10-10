use crate::{
    annotation_canvas::{AnnotationCanvas, DraftState},
    editor::{Message, shortcuts},
    viewport::Viewport,
};
use iced::{
    Event, Rectangle,
    mouse::{Button, Cursor, Event as MouseEvent},
    widget::Action,
};

impl AnnotationCanvas<'_> {
    pub(super) fn handle_event(
        &self,
        state: &mut DraftState,
        event: &Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        if let Some(message) = shortcuts::history(event) {
            if state.take().is_some() {
                self.cache.clear();
                return Some(Action::request_redraw().and_capture());
            }
            return Some(Action::publish(message).and_capture());
        }

        if let Some(action) = self.handle_cancel(state, event) {
            return Some(action);
        }

        let viewport = Viewport::new(self.image_size, bounds.size())?;
        let position = cursor.position_from(bounds.position());

        match event {
            Event::Mouse(MouseEvent::ButtonPressed(Button::Left)) => {
                let start = viewport.image_position(position?)?;
                *state = Some(self.new_draft(start));
                Some(self.redraw())
            }
            Event::Mouse(MouseEvent::CursorMoved { position }) => {
                self.record_motion(state, *position, bounds, viewport)
            }
            Event::Mouse(MouseEvent::ButtonReleased(Button::Left)) => {
                let mut draft = state.take()?;
                if let Some(point) = position.and_then(|p| viewport.clamped_image_position(p)) {
                    draft.record(point);
                }

                Some(self.finish_draft(draft))
            }
            _ => None,
        }
    }
}
