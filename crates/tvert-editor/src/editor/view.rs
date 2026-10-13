use crate::{
    annotation_canvas::AnnotationCanvas,
    editor::{Editor, Message},
    image_canvas::ImageCanvas,
    toolbar,
};
use iced::{
    Color, Padding,
    widget::{canvas, column, container, opaque, space, stack, text},
};
use iced::{Element, Fill};
use std::error::Error as StdError;

impl Editor {
    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![stack![
            self.image_preview(),
            toolbar::view(
                self.drawing_tool(),
                self.drawing_tool().and_then(|tool| toolbar::size_control(
                    tool,
                    self.selected_size(),
                    self.size_input.as_deref(),
                    self.initial_settings
                )),
                self.stroke_selection,
                self.color_palette_open,
                &self.color_hex,
                !self.undo_stack.is_empty(),
                !self.redo_stack.is_empty(),
            ),
        ]];

        if let Some(error) = &self.error {
            content = content.push(text(error_message(error)).style(text::danger));
        }

        container(content).width(Fill).height(Fill).into()
    }

    fn image_preview(&self) -> Element<'_, Message> {
        let padding = Padding::ZERO.top(toolbar::HEIGHT);
        let picture = canvas(ImageCanvas {
            preview: &self.preview,
            image_size: self.image_size,
            navigation: self.navigation,
            padding,
        })
        .width(Fill)
        .height(Fill);

        let annotations = canvas(AnnotationCanvas {
            image_size: self.image_size,
            navigation: self.navigation,
            padding,
            tool: self.tool,
            mode: self.mode,
            size: self.selected_size(),
            stroke_color: Color::from(self.stroke_selection),
            edits: &self.edits,
            cache: &self.cache,
        })
        .width(Fill)
        .height(Fill);

        // Images ignore geometry clip bounds, and a stack clips only the
        // layers above its first, so an empty base keeps both canvases clipped
        let mut preview = stack![space().width(Fill).height(Fill), picture, annotations]
            .width(Fill)
            .height(Fill)
            .clip(true);

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
    use std::assert_matches;
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
        let limits = layout::Limits::new(Size::ZERO, Size::new(200.0, 200.0));
        let node = preview
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let layout = Layout::new(&node);
        let (start, end) = (Point::new(50.0, 100.0), Point::new(150.0, 150.0));
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
        assert_matches!(
            drag_across_preview(&editor(false)).as_slice(),
            [Message::AddEdit(_)]
        );
        assert_eq!(drag_across_preview(&editor(true)), []);
    }
}
