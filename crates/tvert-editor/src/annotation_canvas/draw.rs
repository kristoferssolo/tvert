use crate::{
    annotation::AnnotationDraft,
    annotation_canvas::{AnnotationCanvas, Gesture, GestureState},
    constraint::Constraint,
    editor::Message,
    viewport::Viewport,
};
use iced::{Point, Rectangle, mouse::Cursor, widget::Action};

impl AnnotationCanvas<'_> {
    pub(super) fn record_motion(
        &self,
        draft: &mut AnnotationDraft,
        point: Point,
        viewport: Viewport,
        shift: bool,
    ) -> Option<Action<Message>> {
        draft.record(draft_position(draft, point, viewport, shift)?);
        Some(self.redraw())
    }

    /// Applies or releases a draft's or crop's Shift constraint without
    /// waiting for the pointer to move
    pub(super) fn constrain_draft(
        &self,
        state: &mut GestureState,
        shift: bool,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        let viewport = self.viewport(bounds)?;
        let point = cursor.position_from(bounds.position())?;
        match state.as_mut()? {
            Gesture::Draw(draft) => {
                Constraint::for_tool(draft.tool)?;
                draft.record(draft_position(draft, point, viewport, shift)?);
            }
            Gesture::Crop(draft) => {
                self.drag_crop(draft, point, viewport, shift)?;
            }
            Gesture::Move(_) | Gesture::Pan(_) => return None,
        }
        self.cache.clear();
        Some(Action::request_redraw())
    }
}

/// Maps a canvas `point` to the draft's next image position, constrained
/// relative to its start while Shift is held
pub(super) fn draft_position(
    draft: &AnnotationDraft,
    point: Point,
    viewport: Viewport,
    shift: bool,
) -> Option<Point> {
    let end = viewport.clamped_image_position(point)?;
    let constraint = Constraint::for_tool(draft.tool).filter(|_| shift);
    Some(match (constraint, draft.points.first()) {
        (Some(constraint), Some(&start)) => constraint.apply(start, end, viewport.area()),
        _ => end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        annotation_canvas::CanvasState, mode::Mode, style::RED, tool::Tool, viewport::Navigation,
    };
    use claims::{assert_none, assert_some, assert_some_eq};
    use iced::{
        Event, Padding, Size, event,
        keyboard::{Event as KeyboardEvent, Modifiers},
        mouse::{Button, Event as MouseEvent},
        widget::canvas::{Cache, Program},
    };
    use rstest::rstest;

    const PRESS: Event = Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
    const RELEASE: Event = Event::Mouse(MouseEvent::ButtonReleased(Button::Left));
    /// Fits the image at twice its size, at (20, 140)–(420, 340) on screen
    const BOUNDS: Rectangle = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
    /// Image position (50, 25)
    const START: Point = Point::new(120.0, 190.0);

    fn canvas(cache: &Cache, tool: Tool) -> AnnotationCanvas<'_> {
        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            visible: None,
            crop: None,
            navigation: Navigation::Fit,
            padding: Padding::ZERO,
            tool,
            mode: Mode::Draw,
            size: 4,
            stroke_color: RED,
            edits: &[],
            cache,
        }
    }

    fn dispatch(
        canvas: &AnnotationCanvas<'_>,
        state: &mut CanvasState,
        event: &Event,
        position: Point,
    ) -> Option<(Option<Message>, event::Status)> {
        canvas
            .update(state, event, BOUNDS, Cursor::Available(position))
            .map(|action| {
                let (message, _, status) = action.into_inner();
                (message, status)
            })
    }

    fn modifiers(modifiers: Modifiers) -> Event {
        Event::Keyboard(KeyboardEvent::ModifiersChanged(modifiers))
    }

    #[rstest]
    #[case::square(Tool::Rectangle, Point::new(220.0, 240.0), Point::new(100.0, 75.0))]
    #[case::shortened_square(Tool::Ellipse, Point::new(320.0, 290.0), Point::new(125.0, 100.0))]
    #[case::horizontal_arrow(Tool::Arrow, Point::new(220.0, 200.0), Point::new(100.0, 25.0))]
    #[case::freehand_pencil(Tool::Pencil, Point::new(220.0, 240.0), Point::new(100.0, 50.0))]
    fn shift_held_before_the_press_constrains_the_release(
        #[case] tool: Tool,
        #[case] release: Point,
        #[case] expected: Point,
    ) {
        let cache = Cache::new();
        let canvas = canvas(&cache, tool);
        let mut state = CanvasState::default();

        assert_none!(dispatch(
            &canvas,
            &mut state,
            &modifiers(Modifiers::SHIFT),
            START
        ));
        dispatch(&canvas, &mut state, &PRESS, START);
        let (message, _) = assert_some!(dispatch(&canvas, &mut state, &RELEASE, release));
        assert_some_eq!(
            message,
            Message::AddEdit(AnnotationDraft {
                tool,
                size: 4,
                stroke_color: RED,
                points: vec![Point::new(50.0, 25.0), expected],
            })
        );
    }

    #[test]
    fn toggling_shift_mid_drag_updates_the_endpoint_without_motion() {
        let cache = Cache::new();
        let canvas = canvas(&cache, Tool::Rectangle);
        let mut state = CanvasState::default();
        let position = Point::new(220.0, 240.0);
        dispatch(&canvas, &mut state, &PRESS, START);
        dispatch(
            &canvas,
            &mut state,
            &Event::Mouse(MouseEvent::CursorMoved { position }),
            position,
        );
        let endpoint = |state: &CanvasState| match &state.gesture {
            Some(Gesture::Draw(draft)) => draft.points.last().copied(),
            _ => None,
        };

        let (_, status) = assert_some!(dispatch(
            &canvas,
            &mut state,
            &modifiers(Modifiers::SHIFT),
            position
        ));
        assert_eq!(status, event::Status::Ignored);
        assert_some_eq!(endpoint(&state), Point::new(100.0, 75.0));

        dispatch(
            &canvas,
            &mut state,
            &modifiers(Modifiers::empty()),
            position,
        );
        assert_some_eq!(endpoint(&state), Point::new(100.0, 50.0));
    }
}
