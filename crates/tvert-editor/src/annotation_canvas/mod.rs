mod input;
mod navigate;
mod select;

use crate::{
    annotation::AnnotationDraft,
    annotation_canvas::{navigate::PanGesture, select::MoveGesture},
    edit::Edit,
    editor::Message,
    mode::Mode,
    tool::Tool,
    viewport::{Navigation, Viewport},
};
use iced::{
    Color, Rectangle, Renderer, Size, Theme,
    mouse::{self, Cursor},
    widget::canvas::{Cache, Program},
};

/// A pointer gesture in progress on the canvas.
///
/// Only one runs at a time, so navigation never changes the mapping
/// beneath a draft or move.
#[derive(Debug)]
pub enum Gesture {
    /// Drawing a new edit with the appearance selected when it began
    Draw(AnnotationDraft),
    /// Pressing or dragging an existing edit
    Move(MoveGesture),
    /// Dragging the view with the middle button
    Pan(PanGesture),
}

type GestureState = Option<Gesture>;

pub struct AnnotationCanvas<'a> {
    pub image_size: Size,
    pub navigation: Navigation,
    pub tool: Tool,
    pub mode: Mode,
    pub size: u16,
    pub stroke_color: Color,
    pub edits: &'a [Edit],
    pub cache: &'a Cache,
}

impl Program<Message> for AnnotationCanvas<'_> {
    type State = GestureState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        self.handle_event(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<iced::widget::canvas::Geometry<Renderer>> {
        let Some(viewport) = self.viewport(bounds) else {
            return Vec::new();
        };

        let geometry = self
            .cache
            .draw_with_bounds(renderer, viewport.bounds, |frame| {
                if let Some(Gesture::Draw(draft)) = state
                    && let Ok(edit) = Edit::try_from(draft.clone())
                {
                    edit.draw_draft(frame, viewport);
                }
                self.draw_selection(frame, viewport, theme);
            });

        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if matches!(state, Some(Gesture::Pan(_))) {
            return mouse::Interaction::Grabbing;
        }
        let over_image = self
            .viewport(bounds)
            .zip(cursor.position_in(bounds))
            .and_then(|(viewport, point)| viewport.image_position(point))
            .is_some();

        if self.mode == Mode::Draw && (over_image || state.is_some()) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }
}

impl AnnotationCanvas<'_> {
    fn viewport(&self, bounds: Rectangle) -> Option<Viewport> {
        Viewport::navigated(self.image_size, bounds, self.navigation)
    }
}
