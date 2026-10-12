mod input;
mod select;

use crate::{
    annotation::AnnotationDraft, edit::Edit, editor::Message, mode::Mode, tool::Tool,
    viewport::Viewport,
};
use iced::{
    Color, Rectangle, Renderer, Size, Theme,
    mouse::{self, Cursor},
    widget::canvas::{Cache, Program},
};

type DraftState = Option<AnnotationDraft>;

pub struct AnnotationCanvas<'a> {
    pub image_size: Size,
    pub tool: Tool,
    pub mode: Mode,
    pub size: u16,
    pub stroke_color: Color,
    pub edits: &'a [Edit],
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
        let Some(viewport) = Viewport::new(self.image_size, bounds) else {
            return Vec::new();
        };

        let geometry = self
            .cache
            .draw_with_bounds(renderer, viewport.bounds, |frame| {
                if let Some(draft) = state
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
        let over_image = Viewport::new(self.image_size, bounds)
            .and_then(|viewport| viewport.image_position(cursor.position_from(bounds.position())?))
            .is_some();

        if self.mode == Mode::Draw && (over_image || state.is_some()) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }
}
