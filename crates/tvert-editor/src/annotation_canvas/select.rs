use crate::{
    annotation_canvas::{AnnotationCanvas, Gesture, GestureState},
    edit,
    editor::{Message, MoveMessage},
    guide,
    mode::Mode,
    viewport::Viewport,
};
use iced::{
    Point, Theme, Vector,
    widget::{Action, canvas::Frame},
};

/// Distance a press travels before it moves an edit, in viewport pixels
const DRAG_THRESHOLD: f32 = 4.0;

/// A press on an edit, which moves it once dragged past [`DRAG_THRESHOLD`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveGesture {
    /// Canvas position of the press
    press: Point,
    /// Image position under the press, which stays under the pointer
    grab: Point,
    /// Latest offset in captured-image pixels, once dragging
    offset: Option<Vector>,
    /// Offset awaiting the next frame's preview
    pending: Option<Vector>,
}

impl MoveGesture {
    /// Previews the edit under a canvas `point`, once past the threshold
    pub(super) fn drag(&mut self, point: Point, viewport: Viewport) -> Option<Action<Message>> {
        self.pending = Some(self.track(point, viewport)?);
        Some(Action::request_redraw().and_capture())
    }

    pub(super) fn present(&mut self) -> Option<Action<Message>> {
        let offset = self.pending.take()?;
        Some(Action::publish(Message::Move(MoveMessage::Drag(offset))))
    }

    /// Finishes a drag at the release `point`, or the last tracked one.
    ///
    /// A release within the threshold is a click that only selects.
    pub(super) fn release(mut self, point: Option<Point>, viewport: Viewport) -> Action<Message> {
        point
            .and_then(|point| self.track(point, viewport))
            .or(self.offset)
            .map_or_else(Action::capture, |offset| {
                Action::publish(Message::Move(MoveMessage::Finish(offset))).and_capture()
            })
    }

    fn track(&mut self, point: Point, viewport: Viewport) -> Option<Vector> {
        if self.offset.is_none() && point.distance(self.press) < DRAG_THRESHOLD {
            return None;
        }
        let current = viewport.clamped_image_position(point)?;
        let offset = Vector::new(current.x - self.grab.x, current.y - self.grab.y);
        self.offset = Some(offset);
        Some(offset)
    }
}

impl AnnotationCanvas<'_> {
    /// Selects the topmost edit under a canvas `point` and prepares to move it,
    /// or clears the selection
    pub(super) fn select_at(
        &self,
        state: &mut GestureState,
        viewport: Viewport,
        point: Point,
    ) -> Action<Message> {
        let index = edit::topmost_hit(self.edits, viewport, point);
        *state = index
            .and_then(|_| viewport.clamped_image_position(point))
            .map(|grab| {
                Gesture::Move(MoveGesture {
                    press: point,
                    grab,
                    offset: None,
                    pending: None,
                })
            });
        Action::publish(Message::SelectEdit(index)).and_capture()
    }

    pub(super) fn draw_selection(&self, frame: &mut Frame, viewport: Viewport, theme: &Theme) {
        let Mode::Select(Some(index)) = self.mode else {
            return;
        };
        if let Some(bounds) = self
            .edits
            .get(index)
            .and_then(|edit| edit.canvas_bounds(viewport))
        {
            guide::selection(frame, bounds, theme.palette());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::{
        annotation::AnnotationDraft, edit::Edit, style::RED, tool::Tool, viewport::Navigation,
    };
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use iced::{
        Event, Padding, Rectangle, Size, event,
        keyboard::{self, Event as KeyboardEvent, Key, Modifiers, key},
        mouse::{Button, Cursor, Event as MouseEvent},
        widget::canvas::{Cache, Program},
        window::Event as WindowEvent,
    };
    use rstest::rstest;

    const PRESS: Event = Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
    const RELEASE: Event = Event::Mouse(MouseEvent::ButtonReleased(Button::Left));
    /// Image (0, 0)–(200, 100) appears at (20, 140)–(420, 340) on screen
    const BOUNDS: Rectangle = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
    /// The middle of the line, at image (100, 50)
    const GRAB: Point = Point::new(220.0, 240.0);

    fn edit(tool: Tool, start: Point, end: Point) -> Edit {
        assert_ok!(Edit::try_from(AnnotationDraft {
            tool,
            size: 4,
            stroke_color: RED,
            points: vec![start, end],
        }))
    }

    fn edits() -> [Edit; 2] {
        [
            edit(Tool::Highlighter, Point::ORIGIN, Point::new(200.0, 100.0)),
            edit(Tool::Line, Point::new(50.0, 50.0), Point::new(150.0, 50.0)),
        ]
    }

    fn canvas<'a>(edits: &'a [Edit], cache: &'a Cache) -> AnnotationCanvas<'a> {
        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            padding: Padding::ZERO,
            navigation: Navigation::Fit,
            tool: Tool::Rectangle,
            mode: Mode::Select(None),
            size: 4,
            stroke_color: RED,
            edits,
            cache,
        }
    }

    fn dispatch(state: &mut GestureState, event: &Event, cursor: Option<Point>) -> Option<Message> {
        let edits = edits();
        let cache = Cache::new();
        canvas(&edits, &cache)
            .update(
                state,
                event,
                BOUNDS,
                cursor.map_or(Cursor::Unavailable, Cursor::Available),
            )
            .and_then(|action| action.into_inner().0)
    }

    fn motion(position: Point) -> Event {
        Event::Mouse(MouseEvent::CursorMoved { position })
    }

    fn frame() -> Event {
        Event::Window(WindowEvent::RedrawRequested(Instant::now()))
    }

    #[rstest]
    #[case::overlapping(GRAB, Some(1))]
    #[case::beneath(Point::new(40.0, 160.0), Some(0))]
    #[case::letterbox(Point::new(220.0, 60.0), None)]
    fn clicks_select_the_topmost_edit_or_clear(
        #[case] click: Point,
        #[case] expected: Option<usize>,
    ) {
        let edits = edits();
        let cache = Cache::new();
        let canvas = canvas(&edits, &cache);
        let mut state = None;

        let action =
            assert_some!(canvas.update(&mut state, &PRESS, BOUNDS, Cursor::Available(click)));
        let (message, _, status) = action.into_inner();

        assert_some_eq!(message, Message::SelectEdit(expected));
        assert_eq!(status, event::Status::Captured);
        assert_eq!(state.is_some(), expected.is_some());
        assert_none!(dispatch(&mut state, &RELEASE, Some(click)));
        assert_none!(&state);
        assert_none!(canvas.update(
            &mut state,
            &PRESS,
            BOUNDS,
            Cursor::Available(Point::new(5.0, 5.0))
        ));
    }

    #[test]
    fn drags_by_image_pixels_past_the_threshold_keeping_the_grab_offset() {
        let mut state = None;
        assert_some!(dispatch(&mut state, &PRESS, Some(GRAB)));

        let nudge = Point::new(222.0, 241.0);
        assert_none!(dispatch(&mut state, &motion(nudge), Some(nudge)));

        assert_none!(dispatch(&mut state, &frame(), None));

        let drag = Point::new(245.0, 265.0);
        assert_none!(dispatch(&mut state, &motion(drag), Some(drag)));
        assert_some_eq!(
            dispatch(&mut state, &frame(), None),
            Message::Move(MoveMessage::Drag(Vector::new(12.5, 12.5)))
        );
        assert_none!(dispatch(&mut state, &motion(GRAB), Some(GRAB)));
        assert_some_eq!(
            dispatch(&mut state, &frame(), None),
            Message::Move(MoveMessage::Drag(Vector::ZERO))
        );

        assert_some_eq!(
            dispatch(&mut state, &RELEASE, Some(Point::new(320.0, 240.0))),
            Message::Move(MoveMessage::Finish(Vector::new(50.0, 0.0)))
        );
        assert_none!(&state);
    }

    #[test]
    fn previews_only_the_latest_offset_once_per_frame() {
        let mut state = None;
        assert_some!(dispatch(&mut state, &PRESS, Some(GRAB)));
        for x in [240.0, 260.0, 280.0, 300.0] {
            let position = Point::new(x, 240.0);
            assert_none!(dispatch(&mut state, &motion(position), Some(position)));
        }

        assert_some_eq!(
            dispatch(&mut state, &frame(), None),
            Message::Move(MoveMessage::Drag(Vector::new(40.0, 0.0)))
        );
        assert_none!(dispatch(&mut state, &frame(), None));
    }

    #[test]
    fn finishes_at_the_last_offset_when_the_cursor_is_unavailable() {
        let mut state = None;
        let drag = Point::new(320.0, 240.0);
        assert_some!(dispatch(&mut state, &PRESS, Some(GRAB)));
        assert_none!(dispatch(&mut state, &motion(drag), Some(drag)));

        assert_some_eq!(
            dispatch(&mut state, &RELEASE, None),
            Message::Move(MoveMessage::Finish(Vector::new(50.0, 0.0)))
        );
    }

    #[rstest]
    #[case(Event::Mouse(MouseEvent::ButtonPressed(Button::Right)))]
    #[case(Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Named(key::Named::Escape),
        modified_key: Key::Named(key::Named::Escape),
        physical_key: key::Physical::Code(key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    }))]
    #[case(Event::Window(WindowEvent::Unfocused))]
    fn cancelling_a_drag_restores_it_without_finishing(#[case] cancel: Event) {
        let mut state = None;
        let drag = Point::new(320.0, 240.0);
        assert_some!(dispatch(&mut state, &PRESS, Some(GRAB)));
        assert_none!(dispatch(&mut state, &motion(drag), Some(drag)));

        assert_some_eq!(
            dispatch(&mut state, &cancel, None),
            Message::Move(MoveMessage::Cancel)
        );
        assert_none!(&state);
        assert_none!(dispatch(&mut state, &RELEASE, Some(drag)));
    }
}
