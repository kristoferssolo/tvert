use crate::{
    annotation_canvas::AnnotationCanvas, figure::Annotation, message::Message,
    stroke_width::StrokeWidth, tool::Tool, toolbar,
};
use iced::widget::{canvas, column, container, image, stack};
use iced::{
    ContentFit, Element, Fill, Size,
    widget::{canvas::Cache, image::Handle},
};

pub struct Editor {
    preview: Handle,
    image_size: Size,
    tool: Tool,
    stroke_width: StrokeWidth,
    figures: Vec<Annotation>,
    undone_figures: Vec<Annotation>,
    annotation_cache: Cache,
}

impl Editor {
    pub fn new(preview: Handle, image_size: Size) -> Self {
        Self {
            preview,
            image_size,
            tool: Tool::default(),
            stroke_width: StrokeWidth::default(),
            figures: Vec::new(),
            undone_figures: Vec::new(),
            annotation_cache: Cache::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::AddFigure {
                tool,
                stroke_width,
                start,
                end,
            } => {
                let Some(figure) = tool.figure(start, end) else {
                    return;
                };
                self.figures.push(Annotation {
                    figure,
                    stroke_width,
                });
                self.undone_figures.clear();
            }
            Message::SelectTool(tool) => {
                self.tool = tool;
                return;
            }
            Message::SelectStrokeWidth(stroke_width) => {
                self.stroke_width = stroke_width;
                return;
            }
            Message::Undo => {
                let Some(annotation) = self.figures.pop() else {
                    return;
                };
                self.undone_figures.push(annotation);
            }
            Message::Redo => {
                let Some(anootation) = self.undone_figures.pop() else {
                    return;
                };
                self.figures.push(anootation);
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
            stroke_width: self.stroke_width,
            figures: &self.figures,
            cache: &self.annotation_cache,
        })
        .width(Fill)
        .height(Fill);

        container(
            column![
                container(toolbar::view(
                    self.tool,
                    !self.figures.is_empty(),
                    !self.undone_figures.is_empty()
                ))
                .center_x(Fill),
                container(toolbar::stroke_width_constrols(self.stroke_width)).center_x(Fill),
                stack![picture, annotations].width(Fill).height(Fill),
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
            stroke_width: StrokeWidth::default(),
            start: bounds.position(),
            end: Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
        }
    }

    fn assert_figures(figures: &[Annotation], expected: &[(Tool, Rectangle)]) {
        let viewport = assert_some!(Viewport::new(
            Size::new(100.0, 100.0),
            Size::new(100.0, 100.0),
        ));

        let actual = figures
            .iter()
            .map(|annotation| {
                annotation
                    .figure
                    .path(viewport)
                    .raw()
                    .iter()
                    .collect::<Vec<_>>()
            })
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

    #[test]
    fn stroke_width_changes_preserve_existing_figures_and_history() {
        let mut editor = editor();
        let thin = Message::AddFigure {
            tool: Tool::Line,
            stroke_width: StrokeWidth::Thin,
            start: Point::new(10.0, 10.0),
            end: Point::new(30.0, 10.0),
        };
        let thick = Message::AddFigure {
            tool: Tool::Line,
            stroke_width: StrokeWidth::Thick,
            start: Point::new(10.0, 10.0),
            end: Point::new(30.0, 10.0),
        };

        editor.update(thin);
        editor.update(Message::SelectStrokeWidth(StrokeWidth::Thick));
        editor.update(thick);
        editor.update(Message::Undo);
        editor.update(Message::SelectStrokeWidth(StrokeWidth::Medium));
        editor.update(Message::Redo);

        let widths = editor
            .figures
            .iter()
            .map(|annotation| annotation.stroke_width)
            .collect::<Vec<_>>();

        assert_eq!(widths, [StrokeWidth::Thin, StrokeWidth::Thick]);
        assert_eq!(editor.stroke_width, StrokeWidth::Medium);
        assert!(editor.undone_figures.is_empty());
    }
}
