use crate::{
    annotation_canvas::AnnotationCanvas,
    editor::{Editor, Message},
    toolbar,
};
use iced::widget::{canvas, column, container, image, opaque, space, stack, text};
use iced::{ContentFit, Element, Fill};
use std::error::Error as StdError;

impl Editor {
    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![
            toolbar::view(
                self.tool,
                self.selected_size(),
                self.stroke_color,
                self.color_pallete_open,
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
            stroke_color: self.stroke_color,
            cache: &self.cache,
        })
        .width(Fill)
        .height(Fill);

        let mut preview = stack![picture, annotations].width(Fill).height(Fill);

        // Dismissing the palette must not start a drag on the image
        if self.color_pallete_open {
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
