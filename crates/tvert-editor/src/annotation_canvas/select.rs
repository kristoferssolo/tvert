use crate::{
    annotation_canvas::AnnotationCanvas, edit, editor::Message, guide, mode::Mode,
    viewport::Viewport,
};
use iced::{
    Point, Theme,
    widget::{Action, canvas::Frame},
};

impl AnnotationCanvas<'_> {
    /// Selects the topmost edit under a canvas `point`, or clears the selection
    pub(super) fn select_at(&self, viewport: Viewport, point: Point) -> Action<Message> {
        let index = edit::topmost_hit(self.edits, viewport, point);
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
    use super::*;
    use crate::{annotation::AnnotationDraft, edit::Edit, style::RED, tool::Tool};
    use claims::{assert_none, assert_ok, assert_some};
    use iced::{
        Event, Rectangle, Size, event,
        mouse::{Button, Cursor, Event as MouseEvent},
        widget::canvas::{Cache, Program},
    };
    use rstest::rstest;

    fn edit(tool: Tool, start: Point, end: Point) -> Edit {
        assert_ok!(Edit::try_from(AnnotationDraft {
            tool,
            size: 4,
            stroke_color: RED,
            points: vec![start, end],
        }))
    }

    /// Image (0, 0)–(200, 100) appears at (20, 140)–(420, 340) on screen
    #[rstest]
    #[case::overlapping(Point::new(220.0, 240.0), Some(1))]
    #[case::beneath(Point::new(40.0, 160.0), Some(0))]
    #[case::letterbox(Point::new(220.0, 60.0), None)]
    fn clicks_select_the_topmost_edit_or_clear(
        #[case] click: Point,
        #[case] expected: Option<usize>,
    ) {
        let cache = Cache::new();
        let edits = [
            edit(Tool::Highlighter, Point::ORIGIN, Point::new(200.0, 100.0)),
            edit(Tool::Line, Point::new(50.0, 50.0), Point::new(150.0, 50.0)),
        ];
        let canvas = AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            tool: Tool::Rectangle,
            mode: Mode::Select(None),
            size: 4,
            stroke_color: RED,
            edits: &edits,
            cache: &cache,
        };
        let bounds = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
        let press = Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
        let mut draft = None;

        let action =
            assert_some!(canvas.update(&mut draft, &press, bounds, Cursor::Available(click)));
        let (message, _, status) = action.into_inner();

        assert_eq!(message, Some(Message::SelectEdit(expected)));
        assert_eq!(status, event::Status::Captured);
        assert_none!(&draft);
        assert_none!(canvas.update(
            &mut draft,
            &press,
            bounds,
            Cursor::Available(Point::new(5.0, 5.0))
        ));
    }
}
