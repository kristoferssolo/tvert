use crate::{
    annotation::{Annotation, AnnotationDraft},
    message::Message,
    stroke_width::StrokeWidth,
    tool::Tool,
    viewport::Viewport,
};
use iced::{
    Color, Event, Point, Rectangle, Renderer, Size, Theme,
    keyboard::{
        Event as KeyboardEvent,
        Key::{self},
        key,
    },
    mouse::{self, Button, Cursor, Event as MouseEvent},
    widget::{
        Action,
        canvas::{Cache, Program},
    },
    window::Event as WindowEvent,
};

type DraftState = Option<AnnotationDraft>;

pub struct AnnotationCanvas<'a> {
    pub image_size: Size,
    pub tool: Tool,
    pub stroke_width: StrokeWidth,
    pub stroke_color: Color,
    pub annotations: &'a [Annotation],
    pub cache: &'a Cache,
}

impl Program<Message> for AnnotationCanvas<'_> {
    type State = DraftState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        if let Some(message) = history_shortcut(event) {
            if state.take().is_some() {
                self.cache.clear();
                return Some(Action::request_redraw().and_capture());
            }
            return Some(Action::publish(message).and_capture());
        }

        if let Some(action) = self.handle_canel(state, event) {
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
            Event::Mouse(MouseEvent::CursorMoved { .. }) => {
                let draft = state.as_mut()?;
                draft.record(viewport.clamped_image_position(position?)?);
                Some(self.redraw())
            }
            Event::Mouse(MouseEvent::ButtonReleased(Button::Left)) => {
                let mut draft = state.take()?;
                // self.cache.clear(); // TEST:
                if let Some(point) = position.and_then(|p| viewport.clamped_image_position(p)) {
                    draft.record(point);
                }

                Some(self.finish_draft(draft))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<iced::widget::canvas::Geometry<Renderer>> {
        let Some(viewport) = Viewport::new(self.image_size, bounds.size()) else {
            return Vec::new();
        };

        let geometry = self
            .cache
            .draw_with_bounds(renderer, viewport.bounds, |frame| {
                for annotation in self.annotations {
                    annotation.draw(frame, viewport);
                }

                if let Some(draft) = state
                    && let Ok(annotation) = Annotation::try_from(draft.clone())
                {
                    annotation.draw(frame, viewport);
                }
            });

        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        let over_image = Viewport::new(self.image_size, bounds.size())
            .and_then(|viewport| viewport.image_position(cursor.position_from(bounds.position())?))
            .is_some();

        if over_image || state.is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }
}

impl AnnotationCanvas<'_> {
    fn redraw(&self) -> Action<Message> {
        self.cache.clear();
        Action::request_redraw().and_capture()
    }

    fn new_draft(&self, start: Point) -> AnnotationDraft {
        AnnotationDraft::new(self.tool, self.stroke_width, self.stroke_color, start)
    }

    fn finish_draft(&self, draft: AnnotationDraft) -> Action<Message> {
        self.cache.clear();
        if draft.is_empty() {
            return Action::request_redraw().and_capture();
        }
        Action::publish(Message::AddAnnotation(draft)).and_capture()
    }

    fn handle_canel(&self, state: &mut DraftState, event: &Event) -> Option<Action<Message>> {
        let explicit = matches!(
            event,
            Event::Mouse(MouseEvent::ButtonPressed(Button::Right))
                | Event::Keyboard(KeyboardEvent::KeyPressed {
                    key: Key::Named(key::Named::Escape),
                    ..
                })
        );
        let unfocused = matches!(event, Event::Window(WindowEvent::Unfocused));

        if state.is_none() || !(explicit || unfocused) {
            return None;
        }

        *state = None;
        self.cache.clear();
        let action = Action::request_redraw();
        Some(if explicit {
            action.and_capture()
        } else {
            action
        })
    }
}

fn history_shortcut(event: &Event) -> Option<Message> {
    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Character(character),
        modifiers,
        repeat: false,
        ..
    }) = event
    else {
        return None;
    };

    if !modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    if character.eq_ignore_ascii_case("z") {
        return Some(if modifiers.shift() {
            Message::Redo
        } else {
            Message::Undo
        });
    }

    if !modifiers.shift() && character.eq_ignore_ascii_case("y") {
        return Some(Message::Redo);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::DEFAULT_STROKE_COLOR;
    use claims::{assert_none, assert_some, assert_some_eq};
    use iced::{
        Point,
        keyboard::{self, Modifiers},
        widget::canvas::Program,
    };
    use rstest::rstest;
    use std::assert_matches;

    fn dispatch(draft: &mut DraftState, event: &Event, position: Option<Point>) -> Option<Message> {
        dispatch_with_tool(draft, event, position, Tool::Rectangle)
    }

    fn dispatch_with_tool(
        draft: &mut DraftState,
        event: &Event,
        position: Option<Point>,
        tool: Tool,
    ) -> Option<Message> {
        let cache = Cache::new();

        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            tool,
            stroke_width: StrokeWidth::default(),
            stroke_color: DEFAULT_STROKE_COLOR,
            annotations: &[],
            cache: &cache,
        }
        .update(
            draft,
            event,
            Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0)),
            position.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
        )
        .and_then(|action| action.into_inner().0)
    }

    fn begin(draft: &mut DraftState) {
        assert_none!(dispatch(
            draft,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(Point::new(120.0, 190.0)),
        ));
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

    #[rstest]
    #[case(Point::new(320.0, 290.0), Point::new(150.0, 75.0))]
    #[case(Point::new(620.0, 740.0), Point::new(200.0, 100.0))]
    #[case(Point::ORIGIN, Point::ORIGIN)]
    fn commits_the_release_position_without_a_motion_event(
        #[case] release: Point,
        #[case] expected: Point,
    ) {
        let mut draft = None;
        begin(&mut draft);

        let message = assert_some!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(release),
        ));

        assert_eq!(
            message,
            Message::AddAnnotation(AnnotationDraft {
                tool: Tool::Rectangle,
                stroke_width: StrokeWidth::default(),
                stroke_color: DEFAULT_STROKE_COLOR,
                points: vec![Point::new(50.0, 25.0), expected]
            })
        );
        assert_none!(draft);
    }

    #[test]
    fn uses_the_last_endpoint_when_the_cursor_is_unavailable() {
        let mut draft = None;
        begin(&mut draft);
        let position = Point::new(320.0, 290.0);

        assert_none!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::CursorMoved { position }),
            Some(position),
        ));

        let message = assert_some!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            None,
        ));

        assert_eq!(
            message,
            Message::AddAnnotation(AnnotationDraft {
                tool: Tool::Rectangle,
                stroke_width: StrokeWidth::default(),
                stroke_color: DEFAULT_STROKE_COLOR,
                points: vec![Point::new(50.0, 25.0), Point::new(150.0, 75.0)]
            })
        );
        assert_none!(draft);
    }

    #[rstest]
    #[case(&Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)))]
    #[case(&Event::Window(WindowEvent::Unfocused))]
    #[case(&Event::Keyboard(KeyboardEvent::KeyPressed {
        key: keyboard::Key::Named(keyboard::key::Named::Escape),
        modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    }))]
    fn cancels_without_committing_on_a_later_release(#[case] cancel: &Event) {
        let mut draft = None;
        begin(&mut draft);

        assert_none!(dispatch(&mut draft, cancel, None));
        assert_none!(&draft);
        assert_none!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(Point::new(320.0, 290.0)),
        ));
    }

    #[test]
    fn ignores_presses_in_the_letterbox() {
        let mut draft = None;

        assert_none!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(Point::new(120.0, 90.0)),
        ));
        assert_none!(draft);
    }

    #[test]
    fn ignores_a_click_without_dragging() {
        let mut draft = None;
        begin(&mut draft);

        assert_none!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(Point::new(120.0, 190.0)),
        ));
        assert_none!(draft);
    }

    #[rstest]
    #[case("z", Modifiers::CTRL, Message::Undo)]
    #[case("Z", Modifiers::CTRL, Message::Undo)]
    #[case("y", Modifiers::CTRL, Message::Redo)]
    #[case("Y", Modifiers::CTRL, Message::Redo)]
    #[case("z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
    #[case("Z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
    fn publishes_history_shortcuts_without_a_cursor(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: Message,
    ) {
        let mut draft = None;
        let event = key_press(character, modifiers, false);
        assert_eq!(dispatch(&mut draft, &event, None), Some(expected));
    }

    #[rstest]
    #[case("z", Modifiers::empty(), false)]
    #[case("z", Modifiers::CTRL | Modifiers::ALT, false)]
    #[case("z", Modifiers::CTRL | Modifiers::LOGO, false)]
    #[case("y", Modifiers::CTRL | Modifiers::SHIFT, false)]
    #[case("s", Modifiers::CTRL, false)]
    #[case("z", Modifiers::CTRL, true)]
    fn ignores_unrelated_keys_and_repeats(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] repeat: bool,
    ) {
        let mut draft = None;
        begin(&mut draft);
        assert_none!(dispatch(
            &mut draft,
            &key_press(character, modifiers, repeat),
            None,
        ));
        assert_some!(draft);
    }

    #[rstest]
    #[case("z", Modifiers::CTRL, Message::Undo)]
    #[case("y", Modifiers::CTRL, Message::Redo)]
    #[case("z", Modifiers::CTRL | Modifiers::SHIFT, Message::Redo)]
    fn cancels_the_draft_before_applying_history(
        #[case] character: &str,
        #[case] modifiers: Modifiers,
        #[case] expected: Message,
    ) {
        let mut draft = None;
        begin(&mut draft);
        let event = key_press(character, modifiers, false);
        assert_none!(dispatch(&mut draft, &event, None));
        assert_none!(&draft);
        assert_none!(dispatch(
            &mut draft,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(Point::new(320.0, 290.0)),
        ));
        assert_eq!(dispatch(&mut draft, &event, None), Some(expected));
    }

    #[test]
    fn finishes_with_the_tool_selected_when_the_drag_started() {
        let cache = Cache::new();
        let mut canvas = AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            tool: Tool::Ellipse,
            stroke_width: StrokeWidth::default(),
            stroke_color: DEFAULT_STROKE_COLOR,
            annotations: &[],
            cache: &cache,
        };
        let bounds = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
        let mut draft = None;

        let _ = canvas.update(
            &mut draft,
            &Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            bounds,
            Cursor::Available(Point::new(120.0, 190.0)),
        );

        assert_matches!(
            draft,
            Some(AnnotationDraft {
                tool: Tool::Ellipse,
                ..
            })
        );

        canvas.tool = Tool::Rectangle;
        let position = Point::new(320.0, 290.0);

        let _ = canvas.update(
            &mut draft,
            &Event::Mouse(MouseEvent::CursorMoved { position }),
            bounds,
            Cursor::Available(position),
        );

        let message = canvas
            .update(
                &mut draft,
                &Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
                bounds,
                Cursor::Available(position),
            )
            .and_then(|action| action.into_inner().0);

        assert_eq!(
            message,
            Some(Message::AddAnnotation(AnnotationDraft {
                tool: Tool::Ellipse,
                stroke_width: StrokeWidth::default(),
                stroke_color: DEFAULT_STROKE_COLOR,
                points: vec![Point::new(50.0, 25.0), Point::new(150.0, 75.0),]
            })),
        );
        assert_none!(draft);
    }

    #[rstest]
    #[case(Point::new(320.0, 190.0), Point::new(150.0, 25.0))]
    #[case(Point::new(120.0, 290.0), Point::new(50.0, 75.0))]
    #[case(Point::new(80.0, 170.0), Point::new(30.0, 15.0))]
    fn commits_line_endpoints_without_normalizing_them(#[case] release: Point, #[case] end: Point) {
        let mut draft = None;

        assert_none!(dispatch_with_tool(
            &mut draft,
            &Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            Some(Point::new(120.0, 190.0)),
            Tool::Line,
        ));

        let message = dispatch_with_tool(
            &mut draft,
            &Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
            Some(release),
            Tool::Line,
        );

        let points = assert_some!(message.and_then(|message| match message {
            Message::AddAnnotation(AnnotationDraft {
                tool: Tool::Line,
                points,
                ..
            }) => Some(points),
            _ => None,
        }));

        assert_some_eq!(points.first(), &Point::new(50.0, 25.0));
        assert!(assert_some!(points.last()).distance(end) < 0.001);
        assert_none!(draft);
    }

    #[test]
    fn finishes_with_the_width_selected_when_the_drag_started() {
        let cache = Cache::new();
        let mut canvas = AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            tool: Tool::Line,
            stroke_width: StrokeWidth::Thin,
            stroke_color: DEFAULT_STROKE_COLOR,
            annotations: &[],
            cache: &cache,
        };
        let bounds = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
        let mut draft = None;

        let _ = canvas.update(
            &mut draft,
            &Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            bounds,
            Cursor::Available(Point::new(120.0, 190.0)),
        );

        canvas.stroke_width = StrokeWidth::Thick;

        let message = canvas
            .update(
                &mut draft,
                &Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
                bounds,
                Cursor::Available(Point::new(320.0, 290.0)),
            )
            .and_then(|action| action.into_inner().0);

        assert_matches!(
            message,
            Some(Message::AddAnnotation(AnnotationDraft {
                stroke_width: StrokeWidth::Thin,
                ..
            }))
        );
        assert_none!(draft);
    }

    #[test]
    fn pencil_retains_motion_samples_and_a_return_to_its_start() {
        let mut drag = None;
        let press = Point::new(120.0, 190.0);
        let middle = Point::new(320.0, 190.0);

        assert_none!(dispatch_with_tool(
            &mut drag,
            &Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            Some(press),
            Tool::Pencil,
        ));

        for position in [middle, middle] {
            assert_none!(dispatch_with_tool(
                &mut drag,
                &Event::Mouse(MouseEvent::CursorMoved { position }),
                Some(position),
                Tool::Pencil,
            ));
        }

        let message = dispatch_with_tool(
            &mut drag,
            &Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
            Some(press),
            Tool::Pencil,
        );

        assert_some_eq!(
            message,
            Message::AddAnnotation(AnnotationDraft {
                tool: Tool::Pencil,
                stroke_width: StrokeWidth::default(),
                stroke_color: DEFAULT_STROKE_COLOR,
                points: vec![
                    Point::new(50.0, 25.0),
                    Point::new(150.0, 25.0),
                    Point::new(50.0, 25.0),
                ],
            })
        );
        assert_none!(drag.as_ref());
    }
}
