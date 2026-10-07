use crate::{
    Message,
    viewport::{Viewport, rectangle_between},
};
use iced::{
    Color, Event, Point, Rectangle, Renderer, Size, Theme,
    keyboard::{
        Event as KeyboadrdEvent,
        Key::{self},
        key,
    },
    mouse::{self, Button, Cursor, Event as MouseEvent},
    widget::{
        Action,
        canvas::{Frame, Program, Stroke},
    },
    window::Event as WindowEvent,
};

pub struct AnnotationCanvas<'a> {
    pub image_size: Size,
    pub rectangles: &'a [Rectangle],
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Drag {
    #[default]
    Idle,
    Drawing {
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
        let cancel = matches!(
            event,
            Event::Mouse(MouseEvent::ButtonPressed(Button::Right))
                | Event::Keyboard(KeyboadrdEvent::KeyPressed {
                    key: Key::Named(key::Named::Escape),
                    ..
                })
        );

        if matches!(state, Drag::Drawing { .. })
            && (cancel || matches!(event, Event::Window(WindowEvent::Unfocused)))
        {
            *state = Drag::Idle;
            let action = Action::request_redraw();
            return Some(if cancel { action.and_capture() } else { action });
        }

        let viewport = Viewport::new(self.image_size, bounds.size())?;
        let position = cursor.position_from(bounds.position());

        match (event, *state) {
            (Event::Mouse(MouseEvent::ButtonPressed(Button::Left)), _) => {
                let start = viewport.image_position(position?)?;
                *state = Drag::Drawing { start, end: start };
                Some(Action::request_redraw().and_capture())
            }
            (Event::Mouse(MouseEvent::CursorMoved { .. }), Drag::Drawing { start, .. }) => {
                let end = viewport.clamped_image_position(position?)?;
                *state = Drag::Drawing { start, end };
                Some(Action::request_redraw().and_capture())
            }
            (
                Event::Mouse(MouseEvent::ButtonReleased(Button::Left)),
                Drag::Drawing { start, end },
            ) => {
                *state = Drag::Idle;
                let end = position
                    .and_then(|point| viewport.clamped_image_position(point))
                    .unwrap_or(end);

                Some(rectangle_between(start, end).map_or_else(
                    || Action::request_redraw().and_capture(),
                    |rectangle| Action::publish(Message::AddRectangle(rectangle)).and_capture(),
                ))
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

        let draft = match state {
            Drag::Idle => None,
            Drag::Drawing { start, end } => rectangle_between(*start, *end),
        };
        let mut frame = Frame::with_bounds(renderer, viewport.bounds);
        let stroke = Stroke::default()
            .with_color(Color::from_rgb8(255, 64, 64))
            .with_width(4.0 * viewport.bounds.width / self.image_size.width);

        for rectangle in self.rectangles.iter().copied().chain(draft) {
            let displayed = viewport.canvas_rectangle(rectangle);
            frame.stroke_rectangle(displayed.position(), displayed.size(), stroke);
        }

        vec![frame.into_geometry()]
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use iced::{keyboard, widget::canvas::Program};
    use rstest::rstest;

    fn dispatch(drag: &mut Drag, event: &Event, position: Option<Point>) -> Option<Message> {
        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            rectangles: &[],
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

    #[rstest]
    #[case(
        Point::new(320.0, 290.0),
        Rectangle::new(Point::new(50.0, 25.0), Size::new(100.0, 50.0))
    )]
    #[case(
        Point::new(620.0, 740.0),
        Rectangle::new(Point::new(50.0, 25.0), Size::new(150.0, 75.0))
    )]
    #[case(Point::ORIGIN, Rectangle::with_size(Size::new(50.0, 25.0)))]
    fn commits_the_release_position_without_a_motion_event(
        #[case] release: Point,
        #[case] expected: Rectangle,
    ) {
        let mut drag = Drag::Idle;
        begin(&mut drag);

        let Message::AddRectangle(rectangle) = assert_some!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Some(release),
        ));

        assert_eq!(rectangle, expected);
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

        let Message::AddRectangle(rectangle) = assert_some!(dispatch(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            None,
        ));

        assert_eq!(
            rectangle,
            Rectangle::new(Point::new(50.0, 25.0), Size::new(100.0, 50.0))
        );
        assert!(matches!(drag, Drag::Idle));
    }

    #[rstest]
    #[case(&Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)))]
    #[case(&Event::Window(WindowEvent::Unfocused))]
    #[case(&Event::Keyboard(keyboard::Event::KeyPressed {
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
}
