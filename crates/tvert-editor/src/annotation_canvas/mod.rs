mod crop;
mod draw;
mod input;
mod navigate;
mod select;

use crate::{
    annotation::AnnotationDraft,
    annotation_canvas::{crop::CropDraft, navigate::PanGesture, select::MoveGesture},
    crop::Crop,
    edit::Edit,
    editor::Message,
    mode::Mode,
    tool::Tool,
    viewport::{Navigation, Viewport},
};
use iced::{
    Color, Padding, Rectangle, Renderer, Size, Theme, keyboard,
    mouse::{self, Cursor},
    widget::canvas::{Cache, Frame, Program},
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
    /// Dragging a replacement crop
    Crop(CropDraft),
}

type GestureState = Option<Gesture>;

/// Canvas state retained between events
#[derive(Debug, Default)]
pub struct CanvasState {
    gesture: GestureState,
    /// Whether Shift is held, tracked before a draft begins
    shift: bool,
}

pub struct AnnotationCanvas<'a> {
    pub image_size: Size,
    /// The shown part of the image, or `None` for all of it
    pub visible: Option<Rectangle>,
    pub crop: Option<Crop>,
    pub navigation: Navigation,
    pub padding: Padding,
    pub tool: Tool,
    pub mode: Mode,
    pub size: u16,
    pub stroke_color: Color,
    pub edits: &'a [Edit],
    pub cache: &'a Cache,
}

impl Program<Message> for AnnotationCanvas<'_> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        if let iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.shift = modifiers.shift();
            return self.constrain_draft(&mut state.gesture, state.shift, bounds, cursor);
        }
        self.handle_event(&mut state.gesture, state.shift, event, bounds, cursor)
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
                if let Some(Gesture::Draw(draft)) = &state.gesture
                    && let Ok(edit) = Edit::try_from(draft.clone())
                {
                    edit.draw_draft(frame, viewport);
                }
                self.draw_selection(frame, viewport, theme);
            });

        let mut overlay = Frame::new(renderer, bounds.size());
        self.draw_crop(&mut overlay, viewport, state.gesture.as_ref(), theme);

        vec![geometry, overlay.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if matches!(state.gesture, Some(Gesture::Pan(_))) {
            return mouse::Interaction::Grabbing;
        }
        let over_image = self
            .viewport(bounds)
            .zip(cursor.position_in(bounds))
            .and_then(|(viewport, point)| viewport.image_position(point))
            .is_some();

        if !(over_image || state.gesture.is_some()) {
            return mouse::Interaction::None;
        }
        match self.mode {
            Mode::Draw if self.tool == Tool::Text => mouse::Interaction::Text,
            Mode::Draw | Mode::Crop(_) => mouse::Interaction::Crosshair,
            Mode::Select(_) => mouse::Interaction::None,
        }
    }
}

impl AnnotationCanvas<'_> {
    fn viewport(&self, bounds: Rectangle) -> Option<Viewport> {
        Viewport::navigated(
            self.image_size,
            self.visible,
            bounds,
            self.padding,
            self.navigation,
        )
    }
}
