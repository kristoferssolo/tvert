pub mod color;
mod message;
pub mod shortcuts;
mod view;

pub use message::Message;

use crate::{
    Error, Settings,
    edit::Edit,
    export::ExportAction,
    render::{render_edits, render_image},
    style::{BLOCK_SIZE_RANGE, BLUR_SIGMA_RANGE, DEFAULT_STROKE_COLOR, STROKE_WIDTH_RANGE},
    tool::Tool,
};
use iced::{
    Size, Task,
    widget::{canvas::Cache, image::Handle},
};
use iced_color_triangle::Selection;
use std::rc::Rc;
use tvert_image::CapturedImage;

/// Exports an edited image using the caller's clipboard or save policy
pub type ExportHandler = Rc<dyn Fn(ExportAction, &CapturedImage) -> Result<(), Error>>;

pub struct Editor {
    original: Handle,
    preview: Handle,
    image_size: Size,
    tool: Tool,
    initial_settings: Settings,
    stroke_width: u16,
    block_size: u16,
    blur_sigma: u16,
    stroke_selection: Selection,
    color_palette_open: bool,
    color_hex: String,
    export: ExportHandler,
    error: Option<Error>,
    edits: Vec<Edit>,
    undone_edits: Vec<Edit>,
    cache: Cache,
}

impl Editor {
    pub fn new(
        preview: Handle,
        image_size: Size,
        settings: Settings,
        export: ExportHandler,
    ) -> Self {
        Self {
            original: preview.clone(),
            preview,
            image_size,
            tool: Tool::default(),
            initial_settings: settings,
            stroke_width: settings.stroke_width,
            block_size: settings.pixelation_block_size,
            blur_sigma: settings.blur_sigma,
            stroke_selection: DEFAULT_STROKE_COLOR.into(),
            color_palette_open: false,
            color_hex: color::format_hex(DEFAULT_STROKE_COLOR),
            export,
            error: None,
            edits: Vec::new(),
            undone_edits: Vec::new(),
            cache: Cache::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddEdit(draft) => {
                let Ok(edit) = Edit::try_from(draft) else {
                    return Task::none();
                };
                self.add_edit(edit);
            }
            Message::SelectTool(tool) => {
                self.tool = tool;
                self.close_color_palette();
                return Task::none();
            }
            Message::SelectStrokeColor(stroke_color) => {
                self.select_stroke_color(stroke_color);
                return Task::none();
            }
            Message::SelectStrokeWidth(stroke_width) => {
                self.stroke_width =
                    stroke_width.clamp(*STROKE_WIDTH_RANGE.start(), *STROKE_WIDTH_RANGE.end());
                return Task::none();
            }
            Message::SelectBlockSize(block_size) => {
                self.block_size =
                    block_size.clamp(*BLOCK_SIZE_RANGE.start(), *BLOCK_SIZE_RANGE.end());
                return Task::none();
            }
            Message::SelectBlurSigma(sigma) => {
                self.blur_sigma = sigma.clamp(*BLUR_SIGMA_RANGE.start(), *BLUR_SIGMA_RANGE.end());
                return Task::none();
            }
            Message::ToggleColorPalette => {
                self.color_palette_open = !self.color_palette_open;
                self.color_hex = color::format_hex(self.stroke_selection.into());
                return Task::none();
            }
            Message::CloseColorPalette => {
                self.close_color_palette();
                return Task::none();
            }
            Message::SelectStrokeSelection(selection) => {
                self.select_stroke_selection(selection);
                return Task::none();
            }
            Message::EditColorHex(value) => {
                self.edit_color_hex(value);
                return Task::none();
            }
            Message::Undo => self.undo(),
            Message::Redo => self.redo(),
            Message::Export(action) => return self.export(action),
        }
        self.cache.clear();
        Task::none()
    }

    /// Returns the committed image at full resolution.
    ///
    /// # Errors
    ///
    /// Returns an error if preparing the pixel storage fails.
    pub fn render_image(&self) -> Result<CapturedImage, Error> {
        render_image(&self.preview, &[])
    }

    pub fn export(&mut self, action: ExportAction) -> Task<Message> {
        match self
            .render_image()
            .and_then(|image| (self.export)(action, &image))
        {
            Ok(()) => {
                self.error = None;
                iced::exit()
            }
            Err(error) => {
                self.error = Some(error);
                Task::none()
            }
        }
    }

    fn add_edit(&mut self, edit: Edit) {
        self.edits.push(edit);
        if self.refresh_preview() {
            self.undone_edits.clear();
            return;
        }
        let _ = self.edits.pop();
    }

    fn undo(&mut self) {
        let Some(edit) = self.edits.pop() else {
            return;
        };
        if self.refresh_preview() {
            self.undone_edits.push(edit);
            return;
        }
        self.edits.push(edit);
    }

    fn redo(&mut self) {
        let Some(edit) = self.undone_edits.pop() else {
            return;
        };
        self.edits.push(edit);
        if !self.refresh_preview()
            && let Some(edit) = self.edits.pop()
        {
            self.undone_edits.push(edit);
        }
    }

    fn refresh_preview(&mut self) -> bool {
        match render_edits(&self.original, &self.edits) {
            Ok(image) => {
                let (width, height) = image.size().into();
                self.preview = Handle::from_rgba(width, height, image.into_rgba());
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    const fn selected_size(&self) -> u16 {
        match self.tool {
            Tool::Pixelate => self.block_size,
            Tool::Blur => self.blur_sigma,
            _ => self.stroke_width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotation::AnnotationDraft, viewport::Viewport};
    use claims::{assert_ok, assert_some};
    use iced::{Color, Point, Rectangle};
    use rstest::rstest;

    const SETTINGS: Settings = Settings {
        stroke_width: 4,
        pixelation_block_size: 16,
        blur_sigma: 8,
    };

    fn editor() -> Editor {
        Editor::new(
            Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            Size::new(1.0, 1.0),
            SETTINGS,
            Rc::new(|_, _| Ok(())),
        )
    }

    fn bounds(x: f32) -> Rectangle {
        Rectangle::new(Point::new(x, 0.0), Size::new(10.0, 10.0))
    }

    fn add_figure(tool: Tool, bounds: Rectangle) -> Message {
        Message::AddEdit(AnnotationDraft {
            tool,
            stroke_color: DEFAULT_STROKE_COLOR,
            size: match tool {
                Tool::Pixelate => SETTINGS.pixelation_block_size,
                Tool::Blur => SETTINGS.blur_sigma,
                _ => SETTINGS.stroke_width,
            },
            points: vec![
                bounds.position(),
                Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
            ],
        })
    }

    fn assert_figures(figures: &[Edit], expected: &[(Tool, Rectangle)]) {
        let viewport = assert_some!(Viewport::new(
            Size::new(100.0, 100.0),
            Size::new(100.0, 100.0),
        ));

        let actual = figures
            .iter()
            .map(|edit| {
                let annotation = assert_some!(edit.annotation());
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
        assert_figures(&editor.edits, &[first]);

        let _ = editor.update(Message::Undo);
        assert_figures(&editor.edits, &[]);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[first]);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[first, second]);
        assert_figures(&editor.undone_edits, &[]);
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

        assert_figures(&editor.edits, &[first, replacement]);
        assert_figures(&editor.undone_edits, &[]);
    }

    #[test]
    fn exhausted_history_is_a_noop() {
        let mut editor = editor();

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[]);
        assert_figures(&editor.undone_edits, &[]);

        let figure = (Tool::Rectangle, bounds(10.0));
        let _ = editor.update(add_figure(figure.0, figure.1));

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Undo);
        assert_figures(&editor.edits, &[]);
        assert_figures(&editor.undone_edits, &[figure]);

        let _ = editor.update(Message::Redo);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[figure]);
        assert_figures(&editor.undone_edits, &[]);
    }

    #[test]
    fn selecting_a_tool_preserves_the_redo_history() {
        let mut editor = editor();
        let figure = (Tool::Rectangle, bounds(10.0));

        let _ = editor.update(add_figure(figure.0, figure.1));
        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::SelectTool(Tool::Ellipse));

        assert_eq!(editor.tool, Tool::Ellipse);
        assert_figures(&editor.edits, &[]);
        assert_figures(&editor.undone_edits, &[figure]);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[figure]);
        assert_figures(&editor.undone_edits, &[]);
    }

    #[test]
    fn appearance_changes_preserve_existing_figures_and_history() {
        let mut editor = editor();
        let blue = Color::from_rgb8(64, 128, 255);
        let green = Color::from_rgb8(64, 200, 96);

        let thin = Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            stroke_color: DEFAULT_STROKE_COLOR,
            size: 2,
            points: vec![Point::new(10.0, 10.0), Point::new(30.0, 10.0)],
        });
        let thick = Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            stroke_color: blue,
            size: 8,
            points: vec![Point::new(10.0, 10.0), Point::new(30.0, 10.0)],
        });

        let _ = editor.update(thin);
        let _ = editor.update(Message::SelectStrokeWidth(8));
        let _ = editor.update(Message::SelectStrokeColor(blue));
        let _ = editor.update(thick);
        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::SelectStrokeWidth(4));
        let _ = editor.update(Message::SelectStrokeColor(green));
        let _ = editor.update(Message::Redo);

        let appearances = editor
            .edits
            .iter()
            .map(|edit| {
                let annotation = assert_some!(edit.annotation());
                (annotation.stroke_width, annotation.stroke_color)
            })
            .collect::<Vec<_>>();

        assert_eq!(appearances, [(2, DEFAULT_STROKE_COLOR), (8, blue)]);

        assert_eq!(editor.stroke_width, 4);
        assert!(editor.undone_edits.is_empty());
        assert_eq!(
            Color::from(editor.stroke_selection).into_rgba8(),
            green.into_rgba8()
        );
    }

    #[rstest]
    #[case(Tool::Pixelate)]
    #[case(Tool::Blur)]
    fn pixel_effect_history_restores_pixels_and_preserves_the_original(#[case] tool: Tool) {
        let pixels = (0..512)
            .flat_map(|index| {
                if index % 2 == 0 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect::<Vec<_>>();
        let original = Handle::from_rgba(32, 16, pixels.clone());
        let mut editor = Editor::new(
            original,
            Size::new(32.0, 16.0),
            SETTINGS,
            Rc::new(|_, _| Ok(())),
        );
        let region = Rectangle::new(Point::ORIGIN, Size::new(16.0, 16.0));

        let _ = editor.update(add_figure(tool, region));
        let processed = assert_ok!(editor.render_image());

        assert_ne!(processed.rgba(), pixels);
        assert_eq!(
            assert_ok!(render_image(&editor.original, &[])).rgba(),
            pixels
        );

        let _ = editor.update(Message::Undo);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), pixels);

        let _ = editor.update(Message::Redo);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), processed.rgba());

        let _ = editor.update(Message::Undo);
        let invalid = Rectangle::new(Point::new(40.0, 0.0), Size::new(8.0, 8.0));
        let _ = editor.update(add_figure(tool, invalid));

        assert!(editor.error.is_some());
        assert!(editor.edits.is_empty());
        assert_eq!(editor.undone_edits.len(), 1);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), pixels);

        let _ = editor.update(Message::Redo);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), processed.rgba());
    }
}
