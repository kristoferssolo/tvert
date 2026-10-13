use crate::{
    annotation::AnnotationDraft,
    annotation_canvas::{
        AnnotationCanvas, Gesture, GestureState,
        navigate::{self, PanGesture},
    },
    editor::{Message, MoveMessage, shortcuts},
    mode::Mode,
    viewport::Viewport,
};
use iced::{
    Event, Point, Rectangle,
    keyboard::{Event as KeyboardEvent, Key, key},
    mouse::{Button, Cursor, Event as MouseEvent},
    widget::Action,
    window::Event as WindowEvent,
};

impl AnnotationCanvas<'_> {
    pub(super) fn handle_event(
        &self,
        state: &mut GestureState,
        event: &Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        if let Some(action) = self.handle_cancel(state, event) {
            return Some(action);
        }

        let viewport = self.viewport(bounds)?;
        let position = cursor.position_from(bounds.position());

        match event {
            Event::Mouse(MouseEvent::WheelScrolled { delta }) => {
                navigate::zoom(state, viewport, cursor.position_in(bounds)?, *delta)
            }
            Event::Mouse(MouseEvent::ButtonPressed(Button::Middle)) => {
                PanGesture::begin(state, viewport, cursor.position_in(bounds)?)
            }
            Event::Mouse(MouseEvent::ButtonPressed(Button::Left))
                if !matches!(state, Some(Gesture::Pan(_))) =>
            {
                let point = cursor.position_in(bounds)?;
                match self.mode {
                    Mode::Draw => {
                        let start = viewport.image_position(point)?;
                        *state = Some(Gesture::Draw(self.new_draft(start)));
                        Some(self.redraw())
                    }
                    Mode::Select(_) => Some(self.select_at(state, viewport, point)),
                }
            }
            Event::Mouse(MouseEvent::CursorMoved { position }) => {
                let local = Cursor::Available(*position).position_from(bounds.position())?;
                match state.as_mut()? {
                    Gesture::Draw(draft) => self.record_motion(draft, local, viewport),
                    Gesture::Move(gesture) => gesture.drag(local, viewport),
                    Gesture::Pan(gesture) => gesture.drag(local),
                }
            }
            Event::Mouse(MouseEvent::ButtonReleased(button)) => {
                self.release(state, *button, position, viewport)
            }
            Event::Window(WindowEvent::RedrawRequested(_)) => {
                let Some(Gesture::Move(gesture)) = state else {
                    return None;
                };
                gesture.present()
            }
            _ => None,
        }
    }

    fn redraw(&self) -> Action<Message> {
        self.cache.clear();
        Action::request_redraw().and_capture()
    }

    fn new_draft(&self, start: Point) -> AnnotationDraft {
        AnnotationDraft::new(self.tool, self.size, self.stroke_color, start)
    }

    fn finish_draft(&self, draft: AnnotationDraft) -> Action<Message> {
        self.cache.clear();
        if draft.is_empty() {
            return Action::request_redraw().and_capture();
        }
        Action::publish(Message::AddEdit(draft)).and_capture()
    }

    fn handle_cancel(&self, state: &mut GestureState, event: &Event) -> Option<Action<Message>> {
        let captured = shortcuts::history(event).is_some()
            || matches!(
                event,
                Event::Mouse(MouseEvent::ButtonPressed(Button::Right))
                    | Event::Keyboard(KeyboardEvent::KeyPressed {
                        key: Key::Named(key::Named::Escape),
                        ..
                    })
            );
        let forwarded = shortcuts::tool(event).is_some()
            || matches!(event, Event::Window(WindowEvent::Unfocused));

        if !(captured || forwarded) {
            return None;
        }

        let gesture = state.take()?;
        self.cache.clear();
        let action = match gesture {
            Gesture::Draw(_) | Gesture::Pan(_) => Action::request_redraw(),
            Gesture::Move(_) => Action::publish(Message::Move(MoveMessage::Cancel)),
        };
        Some(if captured {
            action.and_capture()
        } else {
            action
        })
    }

    fn record_motion(
        &self,
        draft: &mut AnnotationDraft,
        point: Point,
        viewport: Viewport,
    ) -> Option<Action<Message>> {
        draft.record(viewport.clamped_image_position(point)?);
        Some(self.redraw())
    }

    fn release(
        &self,
        state: &mut GestureState,
        button: Button,
        position: Option<Point>,
        viewport: Viewport,
    ) -> Option<Action<Message>> {
        match (state.take()?, button) {
            (Gesture::Draw(mut draft), Button::Left) => {
                if let Some(point) =
                    position.and_then(|point| viewport.clamped_image_position(point))
                {
                    draft.record(point);
                }
                Some(self.finish_draft(draft))
            }
            (Gesture::Move(gesture), Button::Left) => Some(gesture.release(position, viewport)),
            (Gesture::Pan(_), Button::Middle) => Some(Action::capture()),
            (gesture, _) => {
                *state = Some(gesture);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{style::RED, tool::Tool, viewport::Navigation};
    use claims::{assert_none, assert_some, assert_some_eq};
    use iced::{
        Color, Padding, Size, event,
        keyboard::{self, Modifiers},
        widget::canvas::{Cache, Program},
    };
    use rstest::rstest;

    const PRESS: Event = Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
    const RELEASE: Event = Event::Mouse(MouseEvent::ButtonReleased(Button::Left));
    const START: Point = Point::new(120.0, 190.0);
    const END: Point = Point::new(320.0, 290.0);
    const BOUNDS: Rectangle = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));

    const STROKE_WIDTH: u16 = 4;

    fn canvas(cache: &Cache, tool: Tool) -> AnnotationCanvas<'_> {
        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            navigation: Navigation::Fit,
            padding: Padding::ZERO,
            tool,
            mode: Mode::Draw,
            size: STROKE_WIDTH,
            stroke_color: RED,
            cache,
            edits: &[],
        }
    }

    fn dispatch_canvas(
        canvas: &AnnotationCanvas<'_>,
        draft: &mut GestureState,
        event: &Event,
        position: Option<Point>,
    ) -> Option<Message> {
        canvas
            .update(
                draft,
                event,
                Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0)),
                position.map_or(Cursor::Unavailable, Cursor::Available),
            )
            .and_then(|action| action.into_inner().0)
    }

    fn capture_status(draft: &mut GestureState, event: &Event) -> event::Status {
        canvas(&Cache::new(), Tool::Rectangle)
            .update(draft, event, BOUNDS, Cursor::Unavailable)
            .map_or(event::Status::Ignored, |action| action.into_inner().2)
    }

    fn dispatch(
        draft: &mut GestureState,
        event: &Event,
        position: Option<Point>,
    ) -> Option<Message> {
        dispatch_with_tool(draft, event, position, Tool::Rectangle)
    }

    fn dispatch_with_tool(
        draft: &mut GestureState,
        event: &Event,
        position: Option<Point>,
        tool: Tool,
    ) -> Option<Message> {
        dispatch_canvas(&canvas(&Cache::new(), tool), draft, event, position)
    }

    fn begin(draft: &mut GestureState) {
        assert_none!(dispatch(draft, &PRESS, Some(START)));
        assert_some!(draft);
    }

    fn key_press(character: &str, modifiers: Modifiers, repeat: bool) -> Event {
        Event::Keyboard(KeyboardEvent::KeyPressed {
            key: Key::Character(character.into()),
            modified_key: Key::Character(character.into()),
            physical_key: key::Physical::Code(key::Code::KeyZ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat,
        })
    }

    fn expected_annotation(tool: Tool, points: Vec<Point>) -> Message {
        Message::AddEdit(AnnotationDraft {
            tool,
            size: STROKE_WIDTH,
            stroke_color: RED,
            points,
        })
    }

    #[rstest]
    #[case(END, Point::new(150.0, 75.0))]
    #[case(Point::new(620.0, 740.0), Point::new(200.0, 100.0))]
    #[case(Point::ORIGIN, Point::ORIGIN)]
    fn commits_the_release_position_without_a_motion_event(
        #[case] release: Point,
        #[case] expected: Point,
    ) {
        let mut draft = None;
        begin(&mut draft);
        let message = dispatch(&mut draft, &RELEASE, Some(release));
        assert_some_eq!(
            message,
            expected_annotation(Tool::Rectangle, vec![Point::new(50.0, 25.0), expected])
        );
        assert_none!(draft);
    }

    #[test]
    fn uses_the_last_endpoint_when_the_cursor_is_unavailable() {
        let mut draft = None;
        begin(&mut draft);
        assert_none!(dispatch(
            &mut draft,
            &Event::Mouse(MouseEvent::CursorMoved { position: END }),
            Some(END)
        ));
        let message = dispatch(&mut draft, &RELEASE, None);
        assert_some_eq!(
            message,
            expected_annotation(
                Tool::Rectangle,
                vec![Point::new(50.0, 25.0), Point::new(150.0, 75.0)]
            )
        );
        assert_none!(draft);
    }

    #[rstest]
    #[case(
        Event::Mouse(MouseEvent::ButtonPressed(Button::Right)),
        event::Status::Captured
    )]
    #[case(Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Named(key::Named::Escape),
        modified_key: Key::Named(key::Named::Escape),
        physical_key: key::Physical::Code(key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    }), event::Status::Captured)]
    #[case(Event::Window(WindowEvent::Unfocused), event::Status::Ignored)]
    #[case(key_press("c", Modifiers::empty(), false), event::Status::Ignored)]
    fn cancels_without_committing_on_a_later_release(
        #[case] cancel: Event,
        #[case] status: event::Status,
    ) {
        let mut draft = None;
        begin(&mut draft);
        assert_eq!(capture_status(&mut draft, &cancel), status);
        assert_none!(&draft);
        assert_none!(dispatch(&mut draft, &RELEASE, Some(END)));
    }

    #[test]
    fn ignores_presses_in_the_letterbox() {
        let mut draft = None;
        assert_none!(dispatch(&mut draft, &PRESS, Some(Point::new(120.0, 90.0))));
        assert_none!(draft);
    }

    #[test]
    fn ignores_a_click_without_dragging() {
        let mut draft = None;
        begin(&mut draft);
        assert_none!(dispatch(&mut draft, &RELEASE, Some(START)));
        assert_none!(draft);
    }

    #[test]
    fn leaves_export_shortcuts_for_the_subscription() {
        let mut draft = None;
        begin(&mut draft);
        assert_none!(dispatch(
            &mut draft,
            &key_press("s", Modifiers::CTRL, false),
            None
        ));
        assert_some!(draft);
    }

    #[test]
    fn history_cancels_a_draft_and_otherwise_reaches_the_editor() {
        let event = key_press("z", Modifiers::CTRL, false);
        let mut draft = None;

        begin(&mut draft);
        assert_eq!(capture_status(&mut draft, &event), event::Status::Captured);
        assert_none!(&draft);
        assert_none!(dispatch(&mut draft, &RELEASE, Some(END)));
        assert_eq!(capture_status(&mut draft, &event), event::Status::Ignored);
    }

    #[test]
    fn retains_the_appearance_selected_when_the_drag_started() {
        let cache = Cache::new();
        let mut canvas = canvas(&cache, Tool::Ellipse);
        canvas.size = 7;
        let mut draft = None;
        assert_none!(dispatch_canvas(&canvas, &mut draft, &PRESS, Some(START)));

        canvas.tool = Tool::Arrow;
        canvas.size = 23;
        canvas.stroke_color = Color::WHITE;

        assert_none!(dispatch_canvas(
            &canvas,
            &mut draft,
            &Event::Mouse(MouseEvent::CursorMoved { position: END }),
            Some(END)
        ));
        let message = dispatch_canvas(&canvas, &mut draft, &RELEASE, Some(END));
        assert_some_eq!(
            message,
            Message::AddEdit(AnnotationDraft {
                tool: Tool::Ellipse,
                size: 7,
                stroke_color: RED,
                points: vec![Point::new(50.0, 25.0), Point::new(150.0, 75.0)],
            })
        );
        assert_none!(draft);
    }

    #[test]
    fn commits_line_endpoints_without_normalizing_them() {
        let mut draft = None;
        assert_none!(dispatch_with_tool(
            &mut draft,
            &PRESS,
            Some(START),
            Tool::Line
        ));
        let message = dispatch_with_tool(
            &mut draft,
            &RELEASE,
            Some(Point::new(80.0, 170.0)),
            Tool::Line,
        );
        let points = assert_some!(message.and_then(|message| match message {
            Message::AddEdit(AnnotationDraft {
                tool: Tool::Line,
                points,
                ..
            }) => Some(points),
            _ => None,
        }));
        assert_some_eq!(points.first(), &Point::new(50.0, 25.0));
        assert!(assert_some!(points.last()).distance(Point::new(30.0, 15.0)) < 0.001);
        assert_none!(draft);
    }

    #[test]
    fn pencil_retains_motion_samples_and_a_return_to_its_start() {
        let mut draft = None;
        let middle = Point::new(320.0, 190.0);
        assert_none!(dispatch_with_tool(
            &mut draft,
            &PRESS,
            Some(START),
            Tool::Pencil
        ));
        for position in [middle, middle] {
            assert_none!(dispatch_with_tool(
                &mut draft,
                &Event::Mouse(MouseEvent::CursorMoved { position }),
                Some(position),
                Tool::Pencil
            ));
        }
        let message = dispatch_with_tool(&mut draft, &RELEASE, Some(START), Tool::Pencil);
        assert_some_eq!(
            message,
            expected_annotation(
                Tool::Pencil,
                vec![
                    Point::new(50.0, 25.0),
                    Point::new(150.0, 25.0),
                    Point::new(50.0, 25.0)
                ]
            )
        );
        assert_none!(draft);
    }

    #[test]
    fn records_each_motion_position_instead_of_the_cursor_snapshot() {
        let mut draft = None;
        assert_none!(dispatch_with_tool(
            &mut draft,
            &PRESS,
            Some(START),
            Tool::Pencil
        ));
        for position in [Point::new(220.0, 190.0), END] {
            assert_none!(dispatch_with_tool(
                &mut draft,
                &Event::Mouse(MouseEvent::CursorMoved { position }),
                Some(END),
                Tool::Pencil
            ));
        }
        let Some(Gesture::Draw(draft)) = draft else {
            panic!("expected a draft");
        };

        assert_eq!(
            draft.points,
            vec![
                Point::new(50.0, 25.0),
                Point::new(100.0, 25.0),
                Point::new(150.0, 75.0)
            ]
        );
    }
}
