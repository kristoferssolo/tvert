use crate::{
    annotation_canvas::{AnnotationCanvas, Gesture, GestureState},
    constraint::Constraint,
    crop::Crop,
    editor::Message,
    guide,
    mode::Mode,
    viewport::Viewport,
};
use iced::{
    Color, Point, Size, Theme,
    widget::{
        Action,
        canvas::{Frame, Path},
    },
};

/// Opacity of the shade over pixels outside the crop while cropping
const CROP_SHADE: f32 = 0.5;

/// A crop being dragged, in captured-image coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropDraft {
    start: Point,
    end: Point,
}

impl CropDraft {
    /// The crop a release would set, snapped as it will be stored
    fn crop(self, image_size: Size) -> Option<Crop> {
        Crop::from_drag(self.start, self.end, image_size)
    }
}

impl AnnotationCanvas<'_> {
    pub(super) fn begin_crop(
        &self,
        state: &mut GestureState,
        viewport: Viewport,
        point: Point,
    ) -> Option<Action<Message>> {
        let start = viewport.image_position(point)?;
        *state = Some(Gesture::Crop(CropDraft { start, end: start }));
        Some(self.redraw())
    }

    pub(super) fn drag_crop(
        &self,
        draft: &mut CropDraft,
        point: Point,
        viewport: Viewport,
        shift: bool,
    ) -> Option<Action<Message>> {
        draft.end = crop_end(*draft, point, viewport, shift)?;
        Some(self.redraw())
    }

    /// Proposes the crop dragged to `point`, or the last tracked position.
    /// A drag covering no whole pixel changes nothing.
    pub(super) fn finish_crop(
        &self,
        mut draft: CropDraft,
        point: Option<Point>,
        viewport: Viewport,
        shift: bool,
    ) -> Action<Message> {
        if let Some(end) = point.and_then(|point| crop_end(draft, point, viewport, shift)) {
            draft.end = end;
        }
        self.cache.clear();
        draft.crop(self.image_size).map_or_else(
            || Action::request_redraw().and_capture(),
            |crop| Action::publish(Message::Crop(crop)).and_capture(),
        )
    }

    /// Shades the image outside the dragged, proposed, or applied crop while
    /// cropping, and otherwise hides everything outside the shown area
    pub(super) fn draw_crop(
        &self,
        frame: &mut Frame,
        viewport: Viewport,
        gesture: Option<&Gesture>,
        theme: &Theme,
    ) {
        let Mode::Crop(proposed) = self.mode else {
            if self.visible.is_some() {
                guide::fill_outside(frame, viewport.visible(), theme.palette().background);
            }
            return;
        };

        let dragged = match gesture {
            Some(Gesture::Crop(draft)) => draft.crop(self.image_size),
            _ => None,
        };
        let Some(crop) = dragged.or(proposed).or(self.crop) else {
            return;
        };
        let bounds = viewport.canvas_rectangle(crop.bounds());
        guide::fill_outside(frame, bounds, Color::BLACK.scale_alpha(CROP_SHADE));
        guide::dashed(
            frame,
            &Path::rectangle(bounds.position(), bounds.size()),
            Color::BLACK,
            Color::WHITE,
        );
    }
}

/// Maps a canvas `point` to the drag's end, square while Shift is held
fn crop_end(draft: CropDraft, point: Point, viewport: Viewport, shift: bool) -> Option<Point> {
    let end = viewport.clamped_image_position(point)?;
    Some(if shift {
        Constraint::Square.apply(draft.start, end, viewport.area())
    } else {
        end
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotation_canvas::CanvasState, style::RED, tool::Tool, viewport::Navigation};
    use claims::{assert_none, assert_some, assert_some_eq};
    use iced::{
        Event, Padding, Rectangle,
        keyboard::{Event as KeyboardEvent, Modifiers},
        mouse::{Button, Cursor, Event as MouseEvent},
        widget::canvas::{Cache, Program},
    };
    use rstest::rstest;

    const PRESS: Event = Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
    const RELEASE: Event = Event::Mouse(MouseEvent::ButtonReleased(Button::Left));
    /// Fits the image at twice its size, at (20, 140)–(420, 340) on screen
    const BOUNDS: Rectangle = Rectangle::new(Point::new(20.0, 40.0), Size::new(400.0, 400.0));
    /// Image position (50, 25)
    const START: Point = Point::new(120.0, 190.0);

    fn canvas(cache: &Cache) -> AnnotationCanvas<'_> {
        AnnotationCanvas {
            image_size: Size::new(200.0, 100.0),
            visible: None,
            crop: None,
            navigation: Navigation::Fit,
            padding: Padding::ZERO,
            tool: Tool::Rectangle,
            mode: Mode::Crop(None),
            size: 4,
            stroke_color: RED,
            edits: &[],
            cache,
        }
    }

    /// Drags from [`START`] to `release` and returns the published message
    fn drag(shift: bool, release: Point) -> Option<Message> {
        let cache = Cache::new();
        let canvas = canvas(&cache);
        let mut state = CanvasState::default();
        let modifiers = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        };
        let events = [
            (
                Event::Keyboard(KeyboardEvent::ModifiersChanged(modifiers)),
                START,
            ),
            (PRESS, START),
            (RELEASE, release),
        ];
        let mut published = None;
        for (event, position) in events {
            if let Some(action) =
                canvas.update(&mut state, &event, BOUNDS, Cursor::Available(position))
            {
                published = action.into_inner().0;
            }
        }
        assert_none!(state.gesture);
        published
    }

    fn crop(start: Point, end: Point) -> Crop {
        assert_some!(Crop::from_drag(start, end, Size::new(200.0, 100.0)))
    }

    #[rstest]
    #[case::snapped(false, Point::new(320.6, 290.6), Point::new(150.0, 75.0))]
    #[case::square(true, Point::new(220.0, 240.0), Point::new(100.0, 75.0))]
    #[case::clipped(false, Point::new(900.0, 900.0), Point::new(200.0, 100.0))]
    fn a_crop_drag_proposes_a_crop(
        #[case] shift: bool,
        #[case] release: Point,
        #[case] end: Point,
    ) {
        assert_some_eq!(
            drag(shift, release),
            Message::Crop(crop(Point::new(50.0, 25.0), end))
        );
    }

    #[test]
    fn a_drag_within_one_pixel_proposes_nothing() {
        assert_none!(drag(false, Point::new(120.6, 190.6)));
    }
}
