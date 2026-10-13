use crate::{
    editor::Message,
    viewport::{Navigation, Viewport},
};
use iced::{
    Padding, Renderer, Size, Theme,
    advanced::graphics::geometry::Frame,
    widget::{
        canvas::{Image, Program},
        image::{FilterMethod, Handle},
    },
};

/// Draws the committed image through the same [`Viewport`] as its overlays
pub struct ImageCanvas<'a> {
    pub preview: &'a Handle,
    pub image_size: Size,
    pub navigation: Navigation,
    pub padding: Padding,
}

impl Program<Message> for ImageCanvas<'_> {
    type State = ();
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::advanced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry<Renderer>> {
        let Some(viewport) =
            Viewport::navigated(self.image_size, bounds, self.padding, self.navigation)
        else {
            return Vec::new();
        };

        // Magnified pixels stay sharp for precise placement
        let filter_method = if viewport.canvas_length(1.0) > 1.0 {
            FilterMethod::Nearest
        } else {
            FilterMethod::Linear
        };

        let mut frame = Frame::new(renderer, bounds.size());
        frame.draw_image(
            viewport.placement(),
            Image::new(self.preview.clone()).filter_method(filter_method),
        );
        vec![frame.into_geometry()]
    }
}
