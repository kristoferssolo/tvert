use crate::{annotation_canvas::AnnotationCanvas, figure::Figure, message::Message, tool::Tool};
use iced::widget::{button, canvas, column, container, image, pick_list, row, space, stack, text};
use iced::{
    ContentFit, Element, Fill, Size,
    widget::{canvas::Cache, image::Handle},
};
use strum::VariantArray;

pub struct Editor {
    preview: Handle,
    image_size: Size,
    tool: Tool,
    figures: Vec<Box<dyn Figure>>,
    undone_figures: Vec<Box<dyn Figure>>,
    annotation_cache: Cache,
}

impl Editor {
    pub fn new(preview: Handle, image_size: Size) -> Self {
        Self {
            preview,
            image_size,
            tool: Tool::default(),
            figures: Vec::new(),
            undone_figures: Vec::new(),
            annotation_cache: Cache::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::AddFigure { tool, start, end } => {
                let Some(figure) = tool.figure(start, end) else {
                    return;
                };
                self.figures.push(figure);
                self.undone_figures.clear();
            }
            Message::SelectTool(tool) => {
                self.tool = tool;
                return;
            }
            Message::Undo => {
                let Some(figure) = self.figures.pop() else {
                    return;
                };
                self.undone_figures.push(figure);
            }
            Message::Redo => {
                let Some(figure) = self.undone_figures.pop() else {
                    return;
                };
                self.figures.push(figure);
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
            tool: self.tool,
            figures: &self.figures,
            cache: &self.annotation_cache,
        })
        .width(Fill)
        .height(Fill);

        container(
            column![
                row![
                pick_list(
                    Tool::VARIANTS,
                    Some(self.tool),
                    Message::SelectTool
                    ),
                    space().width(Fill),
                    button("Undo")
                        .on_press_maybe((!self.figures.is_empty()).then_some(Message::Undo)),
                    button("Redo")
                        .on_press_maybe((!self.undone_figures.is_empty()).then_some(Message::Redo)),
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
    use crate::viewport::Viewport;
    use claims::assert_some;
    use iced::{Point, Rectangle};

    fn editor() -> Editor {
        Editor::new(
            Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            Size::new(1.0, 1.0),
        )
    }

    fn bounds(x: f32) -> Rectangle {
        Rectangle::new(Point::new(x, 0.0), Size::new(10.0, 10.0))
    }

    fn add_figure(tool: Tool, bounds: Rectangle) -> Message {
        Message::AddFigure {
            tool,
            start: bounds.position(),
            end: Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
        }
    }

    fn assert_figures(figures: &[Box<dyn Figure>], expected: &[(Tool, Rectangle)]) {
        let viewport = assert_some!(Viewport::new(
            Size::new(100.0, 100.0),
            Size::new(100.0, 100.0),
        ));

        let actual = figures
            .iter()
            .map(|figure| figure.path(viewport).raw().iter().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let expected = expected
            .iter()
            .map(|&(tool, bounds)| {
                assert_some!(tool.figure(
                    bounds.position(),
                    Point::new(bounds.x + bounds.width, bounds.y + bounds.height)
                ))
                .path(viewport)
                .raw()
                .iter()
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }

    #[test]
    fn undo_and_redo_restore_the_original_drawing_order() {
        let mut editor = editor();
        let first = (Tool::Rectangle, bounds(10.0));
        let second = (Tool::Arrow, bounds(20.0));

        for (tool, bounds) in [first, second] {
            editor.update(add_figure(tool, bounds));
        }

        editor.update(Message::Undo);
        assert_figures(&editor.figures, &[first]);

        editor.update(Message::Undo);
        assert_figures(&editor.figures, &[]);

        editor.update(Message::Redo);
        assert_figures(&editor.figures, &[first]);

        editor.update(Message::Redo);
        assert_figures(&editor.figures, &[first, second]);
        assert_figures(&editor.undone_figures, &[]);
    }

    #[test]
    fn a_new_figure_after_undo_discards_the_redo_branch() {
        let mut editor = editor();
        let first = (Tool::Rectangle, bounds(10.0));
        let second = (Tool::Ellipse, bounds(20.0));
        let replacement = (Tool::Ellipse, bounds(30.0));

        for (tool, bounds) in [first, second] {
            editor.update(add_figure(tool, bounds));
        }

        editor.update(Message::Undo);
        editor.update(add_figure(replacement.0, replacement.1));
        editor.update(Message::Redo);

        assert_figures(&editor.figures, &[first, replacement]);
        assert_figures(&editor.undone_figures, &[]);
    }

    #[test]
    fn exhausted_history_is_a_noop() {
        let mut editor = editor();

        editor.update(Message::Undo);
        editor.update(Message::Redo);
        assert_figures(&editor.figures, &[]);
        assert_figures(&editor.undone_figures, &[]);

        let figure = (Tool::Rectangle, bounds(10.0));
        editor.update(add_figure(figure.0, figure.1));

        editor.update(Message::Undo);
        editor.update(Message::Undo);
        assert_figures(&editor.figures, &[]);
        assert_figures(&editor.undone_figures, &[figure]);

        editor.update(Message::Redo);
        editor.update(Message::Redo);
        assert_figures(&editor.figures, &[figure]);
        assert_figures(&editor.undone_figures, &[]);
    }

    #[test]
    fn selecting_a_tool_preserves_the_redo_history() {
        let mut editor = editor();
        let figure = (Tool::Rectangle, bounds(10.0));

        editor.update(add_figure(figure.0, figure.1));
        editor.update(Message::Undo);
        editor.update(Message::SelectTool(Tool::Ellipse));

        assert_eq!(editor.tool, Tool::Ellipse);
        assert_figures(&editor.figures, &[]);
        assert_figures(&editor.undone_figures, &[figure]);

        editor.update(Message::Redo);
        assert_figures(&editor.figures, &[figure]);
        assert_figures(&editor.undone_figures, &[]);
    }
}
