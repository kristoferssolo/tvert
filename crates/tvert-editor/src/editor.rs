use crate::{
    Error,
    annotation::{Annotation, DEFAULT_STROKE_COLOR},
    annotation_canvas::AnnotationCanvas,
    export::ExportAction,
    message::Message,
    render::render_image,
    stroke_width::StrokeWidth,
    tool::Tool,
    toolbar,
};
use iced::widget::{canvas, column, container, image, opaque, space, stack, text};
use iced::{
    Color, ContentFit, Element, Fill, Size, Task,
    widget::{canvas::Cache, image::Handle},
};
use std::{error::Error as StdError, rc::Rc};
use tvert_image::CapturedImage;

/// Exports an edited image using the caller's clipboard or save policy
pub type ExportHandler = Rc<dyn Fn(ExportAction, &CapturedImage) -> Result<(), Error>>;

pub struct Editor {
    preview: Handle,
    image_size: Size,
    tool: Tool,
    stroke_width: StrokeWidth,
    stroke_color: Color,
    export: ExportHandler,
    export_error: Option<Error>,
    annotations: Vec<Annotation>,
    undone_annotations: Vec<Annotation>,
    annotation_cache: Cache,
    color_pallete_open: bool,
}

impl Editor {
    pub fn new(preview: Handle, image_size: Size, export: ExportHandler) -> Self {
        Self {
            preview,
            image_size,
            tool: Tool::default(),
            stroke_width: StrokeWidth::default(),
            stroke_color: DEFAULT_STROKE_COLOR,
            export,
            export_error: None,
            annotations: Vec::new(),
            undone_annotations: Vec::new(),
            annotation_cache: Cache::new(),
            color_pallete_open: false,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddAnnotation(draft) => {
                let Ok(annotation) = Annotation::try_from(draft) else {
                    return Task::none();
                };
                self.annotations.push(annotation);
                self.undone_annotations.clear();
            }
            Message::SelectTool(tool) => {
                self.tool = tool;
                return Task::none();
            }
            Message::SelectStrokeWidth(stroke_width) => {
                self.stroke_width = stroke_width;
                return Task::none();
            }
            Message::SelectStrokeColor(stroke_color) => {
                self.stroke_color = stroke_color;
                self.color_pallete_open = false;
                return Task::none();
            }
            Message::ToggleColorPallete => {
                self.color_pallete_open = !self.color_pallete_open;
                return Task::none();
            }
            Message::CloseColorPallete => {
                self.color_pallete_open = false;
                return Task::none();
            }
            Message::Undo => {
                let Some(annotation) = self.annotations.pop() else {
                    return Task::none();
                };
                self.undone_annotations.push(annotation);
            }
            Message::Redo => {
                let Some(anootation) = self.undone_annotations.pop() else {
                    return Task::none();
                };
                self.annotations.push(anootation);
            }
            Message::Export(action) => return self.export(action),
        }
        self.annotation_cache.clear();
        Task::none()
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
            stroke_color: self.stroke_color,
            annotations: &self.annotations,
            cache: &self.annotation_cache,
        })
        .width(Fill)
        .height(Fill);

        let mut preview = stack![picture, annotations].width(Fill).height(Fill);

        // Dismissing the palette must not start a drag on the image
        if self.color_pallete_open {
            preview = preview.push(opaque(space().width(Fill).height(Fill)));
        }

        let mut content = column![
            toolbar::view(
                self.tool,
                self.stroke_width,
                self.stroke_color,
                self.color_pallete_open,
                !self.annotations.is_empty(),
                !self.undone_annotations.is_empty()
            ),
            preview
        ]
        .spacing(12);

        if let Some(error) = &self.export_error {
            content = content.push(text(error_message(error)).style(text::danger));
        }

        container(content).width(Fill).height(Fill).into()
    }

    /// Renders committed annotations over the original capture at full resolution.
    ///
    /// # Errors
    ///
    /// Returns an error if preparing the pixels or export viewport fails.
    pub fn render_image(&self) -> Result<CapturedImage, Error> {
        render_image(&self.preview, &self.annotations)
    }

    pub fn export(&mut self, action: ExportAction) -> Task<Message> {
        match self
            .render_image()
            .and_then(|image| (self.export)(action, &image))
        {
            Ok(()) => {
                self.export_error = None;
                iced::exit()
            }
            Err(error) => {
                self.export_error = Some(error);
                Task::none()
            }
        }
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
    use crate::{annotation::AnnotationDraft, viewport::Viewport};
    use claims::assert_some;
    use iced::{Point, Rectangle};

    fn editor() -> Editor {
        Editor::new(
            Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            Size::new(1.0, 1.0),
            Rc::new(|_, _| Ok(())),
        )
    }

    fn bounds(x: f32) -> Rectangle {
        Rectangle::new(Point::new(x, 0.0), Size::new(10.0, 10.0))
    }

    fn add_figure(tool: Tool, bounds: Rectangle) -> Message {
        Message::AddAnnotation(AnnotationDraft {
            tool,
            stroke_width: StrokeWidth::default(),
            stroke_color: DEFAULT_STROKE_COLOR,
            points: vec![
                bounds.position(),
                Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
            ],
        })
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
                assert_some!(tool.figure(vec![
                    bounds.position(),
                    Point::new(bounds.x + bounds.width, bounds.y + bounds.height)
                ]))
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
            let _ = editor.update(add_figure(tool, bounds));
        }

        let _ = editor.update(Message::Undo);
        assert_figures(&editor.annotations, &[first]);

        let _ = editor.update(Message::Undo);
        assert_figures(&editor.annotations, &[]);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.annotations, &[first]);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.annotations, &[first, second]);
        assert_figures(&editor.undone_annotations, &[]);
    }

    #[test]
    fn a_new_figure_after_undo_discards_the_redo_branch() {
        let mut editor = editor();
        let first = (Tool::Rectangle, bounds(10.0));
        let second = (Tool::Ellipse, bounds(20.0));
        let replacement = (Tool::Ellipse, bounds(30.0));

        for (tool, bounds) in [first, second] {
            let _ = editor.update(add_figure(tool, bounds));
        }

        let _ = editor.update(Message::Undo);
        let _ = editor.update(add_figure(replacement.0, replacement.1));
        let _ = editor.update(Message::Redo);

        assert_figures(&editor.annotations, &[first, replacement]);
        assert_figures(&editor.undone_annotations, &[]);
    }

    #[test]
    fn exhausted_history_is_a_noop() {
        let mut editor = editor();

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.annotations, &[]);
        assert_figures(&editor.undone_annotations, &[]);

        let figure = (Tool::Rectangle, bounds(10.0));
        let _ = editor.update(add_figure(figure.0, figure.1));

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Undo);
        assert_figures(&editor.annotations, &[]);
        assert_figures(&editor.undone_annotations, &[figure]);

        let _ = editor.update(Message::Redo);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.annotations, &[figure]);
        assert_figures(&editor.undone_annotations, &[]);
    }

    #[test]
    fn selecting_a_tool_preserves_the_redo_history() {
        let mut editor = editor();
        let figure = (Tool::Rectangle, bounds(10.0));

        let _ = editor.update(add_figure(figure.0, figure.1));
        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::SelectTool(Tool::Ellipse));

        assert_eq!(editor.tool, Tool::Ellipse);
        assert_figures(&editor.annotations, &[]);
        assert_figures(&editor.undone_annotations, &[figure]);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.annotations, &[figure]);
        assert_figures(&editor.undone_annotations, &[]);
    }

    #[test]
    fn appearance_changes_preserve_existing_figures_and_history() {
        let mut editor = editor();
        let blue = Color::from_rgb8(64, 128, 255);
        let green = Color::from_rgb8(64, 200, 96);

        let thin = Message::AddAnnotation(AnnotationDraft {
            tool: Tool::Line,
            stroke_width: StrokeWidth::Thin,
            stroke_color: DEFAULT_STROKE_COLOR,
            points: vec![Point::new(10.0, 10.0), Point::new(30.0, 10.0)],
        });
        let thick = Message::AddAnnotation(AnnotationDraft {
            tool: Tool::Line,
            stroke_width: StrokeWidth::Thick,
            stroke_color: blue,
            points: vec![Point::new(10.0, 10.0), Point::new(30.0, 10.0)],
        });

        let _ = editor.update(thin);
        let _ = editor.update(Message::SelectStrokeWidth(StrokeWidth::Thick));
        let _ = editor.update(Message::SelectStrokeColor(blue));
        let _ = editor.update(thick);
        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::SelectStrokeWidth(StrokeWidth::Medium));
        let _ = editor.update(Message::SelectStrokeColor(green));
        let _ = editor.update(Message::Redo);

        let appearances = editor
            .annotations
            .iter()
            .map(|annotation| (annotation.stroke_width, annotation.stroke_color))
            .collect::<Vec<_>>();

        assert_eq!(
            appearances,
            [
                (StrokeWidth::Thin, DEFAULT_STROKE_COLOR),
                (StrokeWidth::Thick, blue)
            ]
        );

        assert_eq!(editor.stroke_width, StrokeWidth::Medium);
        assert_eq!(editor.stroke_color, green);
        assert!(editor.undone_annotations.is_empty());
    }
}
