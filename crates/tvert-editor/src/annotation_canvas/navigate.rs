use crate::{
    annotation_canvas::{Gesture, GestureState},
    editor::Message,
    viewport::{Navigation, Viewport},
};
use iced::{Point, Vector, mouse::ScrollDelta, widget::Action};

/// Zoom factor for one wheel line
const LINE_ZOOM: f32 = 1.2;

/// Pixel scrolling that zooms as far as one wheel line
const LINE_PIXELS: f32 = 50.0;

/// A middle-button drag that moves the view without editing
#[derive(Debug, Clone, Copy)]
pub struct PanGesture {
    /// Canvas position of the press
    press: Point,
    /// View when the press began, so the image follows the pointer without drift
    start: Viewport,
}

impl PanGesture {
    /// Starts panning from a canvas `point` unless another gesture is running
    pub(super) fn begin(
        state: &mut GestureState,
        viewport: Viewport,
        point: Point,
    ) -> Option<Action<Message>> {
        if state.is_some() {
            return None;
        }
        *state = Some(Gesture::Pan(Self {
            press: point,
            start: viewport,
        }));
        Some(Action::capture())
    }

    /// Keeps the image point under the press beneath a canvas `point`
    pub(super) fn drag(&self, point: Point) -> Option<Action<Message>> {
        let offset = Vector::new(point.x - self.press.x, point.y - self.press.y);
        self.start.pan(offset).map(navigate)
    }
}

pub(super) fn zoom(
    state: &GestureState,
    viewport: Viewport,
    anchor: Point,
    delta: ScrollDelta,
) -> Option<Action<Message>> {
    if state.is_some() {
        return None;
    }
    let lines = match delta {
        ScrollDelta::Lines { y, .. } => y,
        ScrollDelta::Pixels { y, .. } => y / LINE_PIXELS,
    };
    viewport.zoom(anchor, LINE_ZOOM.powf(lines)).map(navigate)
}

fn navigate(navigation: Navigation) -> Action<Message> {
    Action::publish(Message::Navigate(navigation)).and_capture()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        annotation::AnnotationDraft, annotation_canvas::AnnotationCanvas, mode::Mode, style::RED,
        tool::Tool,
    };
    use claims::{assert_none, assert_some, assert_some_eq};
    use iced::{
        Event, Rectangle, Size,
        mouse::{Button, Cursor, Event as MouseEvent},
        widget::canvas::{Cache, Program},
    };
    use std::assert_matches;

    const IMAGE: Size = Size::new(200.0, 100.0);
    /// Fits the image at twice its size, at (20, 140)–(420, 340) on screen
    const BOUNDS: Rectangle = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));

    fn dispatch(
        navigation: Navigation,
        state: &mut GestureState,
        event: MouseEvent,
        cursor: Point,
    ) -> Option<Message> {
        let cache = Cache::new();
        let canvas = AnnotationCanvas {
            image_size: IMAGE,
            navigation,
            tool: Tool::Rectangle,
            mode: Mode::Draw,
            size: 4,
            stroke_color: RED,
            edits: &[],
            cache: &cache,
        };
        canvas
            .update(
                state,
                &Event::Mouse(event),
                BOUNDS,
                Cursor::Available(cursor),
            )
            .and_then(|action| action.into_inner().0)
    }

    fn viewport(navigation: Navigation) -> Viewport {
        assert_some!(Viewport::navigated(IMAGE, BOUNDS, navigation))
    }

    fn image_point(navigation: Navigation, cursor: Point) -> Point {
        assert_some!(
            viewport(navigation)
                .image_position(Point::new(cursor.x - BOUNDS.x, cursor.y - BOUNDS.y))
        )
    }

    #[test]
    fn wheel_zoom_keeps_the_image_point_beneath_the_cursor() {
        let cursor = Point::new(300.0, 210.0);
        let beneath = image_point(Navigation::Fit, cursor);
        let mut navigation = Navigation::Fit;

        for delta in [
            ScrollDelta::Lines { x: 0.0, y: 3.0 },
            ScrollDelta::Pixels { x: 0.0, y: -40.0 },
            ScrollDelta::Pixels { x: 0.0, y: 125.0 },
            ScrollDelta::Lines { x: 0.0, y: -1.0 },
        ] {
            let wheel = MouseEvent::WheelScrolled { delta };
            navigation = match dispatch(navigation, &mut None, wheel, cursor) {
                Some(Message::Navigate(navigation)) => navigation,
                message => panic!("expected navigation, got {message:?}"),
            };

            // The renderer places the image origin on whole image pixels
            let now = image_point(navigation, cursor);
            assert!(
                (now.x - beneath.x).abs() < 1.0 && (now.y - beneath.y).abs() < 1.0,
                "{now:?} drifted from {beneath:?}"
            );
        }
        assert!(
            viewport(navigation).canvas_length(1.0) > viewport(Navigation::Fit).canvas_length(1.0)
        );
    }

    #[test]
    fn middle_drag_pans_without_drawing_and_later_drafts_use_the_panned_view() {
        let start = Navigation::Manual {
            scale: 4.0,
            center: Point::new(100.0, 50.0),
        };
        let panned = Navigation::Manual {
            scale: 4.0,
            center: Point::new(110.0, 45.0),
        };
        let center = BOUNDS.center();
        let drag = Point::new(180.0, 260.0);
        let mut state = None;

        assert_none!(dispatch(
            start,
            &mut state,
            MouseEvent::ButtonPressed(Button::Middle),
            center
        ));
        assert_none!(dispatch(
            start,
            &mut state,
            MouseEvent::ButtonPressed(Button::Left),
            center
        ));
        assert_matches!(state, Some(Gesture::Pan(_)));
        assert_some_eq!(
            dispatch(
                start,
                &mut state,
                MouseEvent::CursorMoved { position: drag },
                drag
            ),
            Message::Navigate(panned)
        );
        assert_none!(dispatch(
            start,
            &mut state,
            MouseEvent::ButtonReleased(Button::Middle),
            drag
        ));
        assert_none!(&state);

        let end = Point::new(380.0, 260.0);
        let wheel = MouseEvent::WheelScrolled {
            delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
        };
        assert_none!(dispatch(
            panned,
            &mut state,
            MouseEvent::ButtonPressed(Button::Left),
            center
        ));
        assert_none!(dispatch(panned, &mut state, wheel, center));
        assert_some_eq!(
            dispatch(
                panned,
                &mut state,
                MouseEvent::ButtonReleased(Button::Left),
                end
            ),
            Message::AddEdit(AnnotationDraft {
                tool: Tool::Rectangle,
                size: 4,
                stroke_color: RED,
                points: vec![Point::new(110.0, 45.0), Point::new(150.0, 50.0)],
            })
        );
    }
}
