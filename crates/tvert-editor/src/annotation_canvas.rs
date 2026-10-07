use crate::{figure::Figure, message::Message, tool::Tool, viewport::Viewport};
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
        canvas::{Cache, Program, Stroke},
    },
    window::Event as WindowEvent,
};

pub struct AnnotationCanvas<'a> {
    pub image_size: Size,
    pub tool: Tool,
    pub figures: &'a [Box<dyn Figure>],
    pub cache: &'a Cache,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Drag {
    #[default]
    Idle,
    Drawing {
        tool: Tool,
        start: Point,
        end: Point,
    },
}

impl Program<Message> for AnnotationCanvas<'_> {
    type State = Drag;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        if let Some(message) = history_shortcut(event) {
            if matches!(state, Drag::Drawing { .. }) {
                *state = Drag::Idle;
                self.cache.clear();
                return Some(Action::request_redraw().and_capture());
            }
            return Some(Action::publish(message).and_capture());
        }

        let cancel = matches!(
            event,
            Event::Mouse(MouseEvent::ButtonPressed(Button::Right))
                | Event::Keyboard(KeyboardEvent::KeyPressed {
                    key: Key::Named(key::Named::Escape),
                    ..
                })
        );

        if matches!(state, Drag::Drawing { .. })
            && (cancel || matches!(event, Event::Window(WindowEvent::Unfocused)))
        {
            *state = Drag::Idle;
            self.cache.clear();
            let action = Action::request_redraw();
            return Some(if cancel { action.and_capture() } else { action });
        }

        let viewport = Viewport::new(self.image_size, bounds.size())?;
        let position = cursor.position_from(bounds.position());

        match (event, *state) {
            (Event::Mouse(MouseEvent::ButtonPressed(Button::Left)), _) => {
                let start = viewport.image_position(position?)?;
                *state = Drag::Drawing {
                    tool: self.tool,
                    start,
                    end: start,
                };
                self.cache.clear();
                Some(Action::request_redraw().and_capture())
            }
            (Event::Mouse(MouseEvent::CursorMoved { .. }), Drag::Drawing { tool, start, .. }) => {
                let end = viewport.clamped_image_position(position?)?;
                *state = Drag::Drawing { tool, start, end };
                self.cache.clear();
                Some(Action::request_redraw().and_capture())
            }
            (
                Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
                Drag::Drawing { tool, start, end },
            ) => {
                *state = Drag::Idle;
                self.cache.clear();

                let end = position
                    .and_then(|point| viewport.clamped_image_position(point))
                    .unwrap_or(end);

                Some(if start == end {
                    Action::request_redraw().and_capture()
                } else {
                    Action::publish(Message::AddFigure { tool, start, end }).and_capture()
                })
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
                let stroke = Stroke::default()
                    .with_color(Color::from_rgb8(255, 64, 64))
                    .with_width(4.0 * viewport.bounds.width / self.image_size.width);

                for figure in self.figures {
                    frame.stroke(&figure.path(viewport), stroke);
                }

                if let Drag::Drawing { tool, start, end } = *state
                    && let Some(figure) = tool.figure(start, end)
                {
                    frame.stroke(&figure.path(viewport), stroke);
                }
            });

        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        state: &Drag,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        let over_image = Viewport::new(self.image_size, bounds.size())
            .and_then(|viewport| viewport.image_position(cursor.position_from(bounds.position())?))
            .is_some();

        if over_image || matches!(state, Drag::Drawing { .. }) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
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
    use claims::{assert_none, assert_some};
    use iced::{
        keyboard::{self, Modifiers},
        widget::canvas::Program,
    };
    use rstest::rstest;

    fn dispatch(drag: &mut Drag, event: &Event, position: Option<Point>) -> Option<Message> {
        dispatch_with_tool(drag, event, position, Tool::Rectangle)
    }

    fn dispatch_with_tool(
        drag: &mut Drag,
        event: &Event,
        position: Option<Point>,
        tool: Tool,
    ) -> Option<Message> {
        let cache = Cache::new();

        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            tool,
            figures: &[],
            cache: &cache,
        }
        .update(
            drag,
            event,
            Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0)),
            position.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
        )
        .and_then(|action| action.into_inner().0)
    }

    fn begin(drag: &mut Drag) {
        assert_none!(dispatch(
            drag,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(Point::new(120.0, 190.0)),
        ));
        assert!(matches!(drag, Drag::Drawing { .. }));
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
        let mut drag = Drag::Idle;
        begin(&mut drag);

        let message = assert_some!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(release),
        ));

        assert_eq!(
            message,
            Message::AddFigure {
                tool: Tool::Rectangle,
                start: Point::new(50.0, 25.0),
                end: expected
            }
        );
        assert!(matches!(drag, Drag::Idle));
    }

    #[test]
    fn uses_the_last_endpoint_when_the_cursor_is_unavailable() {
        let mut drag = Drag::Idle;
        begin(&mut drag);
        let position = Point::new(320.0, 290.0);

        assert_none!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::CursorMoved { position }),
            Some(position),
        ));

        let message = assert_some!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            None,
        ));

        assert_eq!(
            message,
            Message::AddFigure {
                tool: Tool::Rectangle,
                start: Point::new(50.0, 25.0),
                end: Point::new(150.0, 75.0)
            }
        );
        assert!(matches!(drag, Drag::Idle));
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
        let mut drag = Drag::Idle;
        begin(&mut drag);
        assert_none!(dispatch(&mut drag, cancel, None));
        assert!(matches!(drag, Drag::Idle));
        assert_none!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(Point::new(320.0, 290.0)),
        ));
    }

    #[test]
    fn ignores_presses_in_the_letterbox() {
        let mut drag = Drag::Idle;

        assert_none!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(Point::new(120.0, 90.0)),
        ));
        assert!(matches!(drag, Drag::Idle));
    }

    #[test]
    fn ignores_a_click_without_dragging() {
        let mut drag = Drag::Idle;
        begin(&mut drag);

        assert_none!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(Point::new(120.0, 190.0)),
        ));
        assert!(matches!(drag, Drag::Idle));
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
        let mut drag = Drag::Idle;
        let event = key_press(character, modifiers, false);
        assert_eq!(dispatch(&mut drag, &event, None), Some(expected));
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
        let mut drag = Drag::Idle;
        begin(&mut drag);
        assert_none!(dispatch(
            &mut drag,
            &key_press(character, modifiers, repeat),
            None,
        ));
        assert!(matches!(drag, Drag::Drawing { .. }));
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
        let mut drag = Drag::Idle;
        begin(&mut drag);
        let event = key_press(character, modifiers, false);
        assert_none!(dispatch(&mut drag, &event, None));
        assert!(matches!(drag, Drag::Idle));
        assert_none!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(Point::new(320.0, 290.0)),
        ));
        assert_eq!(dispatch(&mut drag, &event, None), Some(expected));
    }

    #[test]
    fn finishes_with_the_tool_selected_when_the_drag_started() {
        let cache = Cache::new();
        let mut canvas = AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            tool: Tool::Ellipse,
            figures: &[],
            cache: &cache,
        };
        let bounds = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
        let mut drag = Drag::Idle;

        let _ = canvas.update(
            &mut drag,
            &Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            bounds,
            Cursor::Available(Point::new(120.0, 190.0)),
        );

        assert!(matches!(
            drag,
            Drag::Drawing {
                tool: Tool::Ellipse,
                ..
            }
        ));

        canvas.tool = Tool::Rectangle;
        let position = Point::new(320.0, 290.0);

        let _ = canvas.update(
            &mut drag,
            &Event::Mouse(MouseEvent::CursorMoved { position }),
            bounds,
            Cursor::Available(position),
        );

        let message = canvas
            .update(
                &mut drag,
                &Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
                bounds,
                Cursor::Available(position),
            )
            .and_then(|action| action.into_inner().0);

        assert_eq!(
            message,
            Some(Message::AddFigure {
                tool: Tool::Ellipse,
                start: Point::new(50.0, 25.0),
                end: Point::new(150.0, 75.0)
            }),
        );
        assert!(matches!(drag, Drag::Idle));
    }

    #[rstest]
    #[case(Point::new(320.0, 190.0), Point::new(150.0, 25.0))]
    #[case(Point::new(120.0, 290.0), Point::new(50.0, 75.0))]
    #[case(Point::new(80.0, 170.0), Point::new(30.0, 15.0))]
    fn commits_line_endpoints_without_normalizing_them(#[case] release: Point, #[case] end: Point) {
        let mut drag = Drag::Idle;

        assert_none!(dispatch_with_tool(
            &mut drag,
            &Event::Mouse(MouseEvent::ButtonPressed(Button::Left)),
            Some(Point::new(120.0, 190.0)),
            Tool::Line,
        ));

        let message = dispatch_with_tool(
            &mut drag,
            &Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
            Some(release),
            Tool::Line,
        );

        let (start, actual_end) = assert_some!(message.and_then(|message| match message {
            Message::AddFigure {
                tool: Tool::Line,
                start,
                end,
            } => Some((start, end)),
            _ => None,
        }));

        assert_eq!(start, Point::new(50.0, 25.0));
        assert!(actual_end.distance(end) < 0.001);
        assert!(matches!(drag, Drag::Idle));
    }
}
