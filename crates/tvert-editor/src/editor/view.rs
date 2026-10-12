use crate::{
    annotation_canvas::AnnotationCanvas,
    editor::{Editor, Message},
    toolbar,
};
use iced::{
    Color,
    widget::{canvas, column, container, image, opaque, space, stack, text},
};
use iced::{ContentFit, Element, Fill};
use std::error::Error as StdError;

impl Editor {
    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![
            toolbar::view(
                self.tool,
                toolbar::size_control(
                    self.tool,
                    self.selected_size(),
                    self.size_input.as_deref(),
                    self.initial_settings
                ),
                self.stroke_selection,
                self.color_palette_open,
                &self.color_hex,
                !self.edits.is_empty(),
                !self.undone_edits.is_empty()
            ),
            self.image_preview()
        ];

        if let Some(error) = &self.error {
            content = content.push(text(error_message(error)).style(text::danger));
        }

        container(content).width(Fill).height(Fill).into()
    }

    fn image_preview(&self) -> Element<'_, Message> {
        let picture = image(&self.preview)
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Contain);

        let annotations = canvas(AnnotationCanvas {
            image_size: self.image_size,
            tool: self.tool,
            size: self.selected_size(),
            stroke_color: Color::from(self.stroke_selection),
            cache: &self.cache,
        })
        .width(Fill)
        .height(Fill);

        let mut preview = stack![picture, annotations].width(Fill).height(Fill);

        // Dismissing the palette must not start a drag on the image
        if self.color_palette_open {
            preview = preview.push(opaque(space().width(Fill).height(Fill)));
        }

        preview.into()
    }
}

fn error_message(error: &dyn StdError) -> String {
    let mut message = error.to_string();
    let mut source = error.source();

    while let Some(error) = source {
        message.push_str(": ");
        message.push_str(&error.to_string());
        source = error.source();
    }

    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, ThemePreference, Tool, style::RED};
    use iced::advanced::{Layout, Shell, clipboard, layout, widget::Tree};
    use iced::mouse::{Button, Cursor, Event as MouseEvent};
    use iced::{Event, Font, Pixels, Point, Renderer, Size, widget::image::Handle};
    use std::rc::Rc;

    fn editor(color_palette_open: bool) -> Editor {
        let mut editor = Editor::new(
            Handle::from_rgba(2, 2, vec![0; 16]),
            Size::new(2.0, 2.0),
            Settings {
                theme: ThemePreference::Dark,
                tool: Tool::Rectangle,
                stroke_color: RED,
                stroke_width: 4,
                pixelation_block_size: 16,
                blur_sigma: 8,
            },
            Rc::new(|_, _| Ok(())),
        );
        editor.color_palette_open = color_palette_open;
        editor
    }

    fn drag_across_preview(editor: &Editor) -> Vec<Message> {
        let renderer = Renderer::new(Font::DEFAULT, Pixels(14.0));
        let mut preview = editor.image_preview();
        let mut tree = Tree::new(preview.as_widget());
        let limits = layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0));
        let node = preview
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let layout = Layout::new(&node);
        let (start, end) = (Point::new(25.0, 25.0), Point::new(75.0, 75.0));
        let mut messages = Vec::new();

        for (event, position) in [
            (Event::Mouse(MouseEvent::ButtonPressed(Button::Left)), start),
            (Event::Mouse(MouseEvent::CursorMoved { position: end }), end),
            (Event::Mouse(MouseEvent::ButtonReleased(Button::Left)), end),
        ] {
            preview.as_widget_mut().update(
                &mut tree,
                &event,
                layout,
                Cursor::Available(position),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &layout.bounds(),
            );
        }

        messages
    }

    #[test]
    fn the_click_that_dismisses_the_palette_does_not_draw() {
        assert!(matches!(
            drag_across_preview(&editor(false)).as_slice(),
            [Message::AddEdit(_)]
        ));
        assert_eq!(drag_across_preview(&editor(true)), []);
    }
}
