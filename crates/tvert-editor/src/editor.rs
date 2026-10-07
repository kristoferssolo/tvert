use crate::{annotation_canvas::AnnotationCanvas, message::Message};
use iced::widget::{button, canvas, column, container, image, row, stack, text};
use iced::{
    ContentFit, Element, Fill, Rectangle, Size,
    widget::{canvas::Cache, image::Handle},
};

pub struct Editor {
    preview: Handle,
    image_size: Size,
    rectangles: Vec<Rectangle>,
    undone_rectangles: Vec<Rectangle>,
    annotation_cache: Cache,
}

impl Editor {
    pub fn new(preview: Handle, image_size: Size) -> Self {
        Self {
            preview,
            image_size,
            rectangles: Vec::new(),
            undone_rectangles: Vec::new(),
            annotation_cache: Cache::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::AddRectangle(rectangle) => {
                self.rectangles.push(rectangle);
                self.undone_rectangles.clear();
            }
            Message::Undo => {
                let Some(rectangle) = self.rectangles.pop() else {
                    return;
                };
                self.undone_rectangles.push(rectangle);
            }
            Message::Redo => {
                let Some(rectangle) = self.undone_rectangles.pop() else {
                    return;
                };
                self.rectangles.push(rectangle);
            }
        }
        self.annotation_cache.clear();
    }

    pub fn view(&self) -> Element<'_, Message> {
        let picture = image(&self.preview)
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Contain);

        let annotations = canvas(AnnotationCanvas {
            image_size: self.image_size,
            rectangles: &self.rectangles,
            cache: &self.annotation_cache,
        })
        .width(Fill)
        .height(Fill);

        container(
            column![
                row![
                    text("Rectangle").width(Fill),
                    button("Undo")
                        .on_press_maybe((!self.rectangles.is_empty()).then_some(Message::Undo)),
                    button("Redo")
                        .on_press_maybe((!self.undone_rectangles.is_empty()).then_some(Message::Redo)),
                ]
                .spacing(8),
                stack![picture, annotations].width(Fill).height(Fill),
                text("Drag to draw. Esc/right-click: cancel. Ctrl+Z: undo. Ctrl+Y/Ctrl+Shift+Z: redo.").size(14),
            ]
            .spacing(12),
        )
        .padding(16)
        .width(Fill)
        .height(Fill)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    fn editor() -> Editor {
        Editor::new(
            Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            Size::new(1.0, 1.0),
        )
    }

    fn rectangle(x: f32) -> Rectangle {
        Rectangle::new(Point::new(x, 0.0), Size::new(10.0, 10.0))
    }

    #[test]
    fn undo_and_redo_restore_the_original_drawing_order() {
        let mut editor = editor();
        let first = rectangle(10.0);
        let second = rectangle(20.0);
        editor.update(Message::AddRectangle(first));
        editor.update(Message::AddRectangle(second));

        editor.update(Message::Undo);
        assert_eq!(editor.rectangles, [first]);
        editor.update(Message::Undo);
        assert_eq!(editor.rectangles, Vec::<Rectangle>::new());

        editor.update(Message::Redo);
        assert_eq!(editor.rectangles, [first]);
        editor.update(Message::Redo);
        assert_eq!(editor.rectangles, [first, second]);
        assert_eq!(editor.undone_rectangles, Vec::<Rectangle>::new());
    }

    #[test]
    fn a_new_rectangle_after_undo_discards_the_redo_branch() {
        let mut editor = editor();
        let first = rectangle(10.0);
        let second = rectangle(20.0);
        let replacement = rectangle(30.0);
        editor.update(Message::AddRectangle(first));
        editor.update(Message::AddRectangle(second));
        editor.update(Message::Undo);
        editor.update(Message::AddRectangle(replacement));
        editor.update(Message::Redo);

        assert_eq!(editor.rectangles, [first, replacement]);
        assert_eq!(editor.undone_rectangles, Vec::<Rectangle>::new());
    }

    #[test]
    fn exhausted_history_is_a_noop() {
        let mut editor = editor();
        editor.update(Message::Undo);
        editor.update(Message::Redo);
        assert_eq!(editor.rectangles, Vec::<Rectangle>::new());
        assert_eq!(editor.undone_rectangles, Vec::<Rectangle>::new());

        let rectangle = rectangle(10.0);
        editor.update(Message::AddRectangle(rectangle));
        editor.update(Message::Undo);
        editor.update(Message::Undo);
        assert_eq!(editor.rectangles, Vec::<Rectangle>::new());
        assert_eq!(editor.undone_rectangles, [rectangle]);

        editor.update(Message::Redo);
        editor.update(Message::Redo);
        assert_eq!(editor.rectangles, [rectangle]);
        assert_eq!(editor.undone_rectangles, Vec::<Rectangle>::new());
    }
}
