pub mod color;
mod history;
mod message;
mod selection;
pub mod shortcuts;
mod size;
mod view;

pub use message::{Message, SizeMessage};

use crate::{
    Error, Settings, color::format_hex, edit::Edit, editor::history::Change, export::ExportAction,
    mode::Mode, render::render_image, size::Sizes, tool::Tool,
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
    mode: Mode,
    initial_settings: Settings,
    sizes: Sizes,
    size_input: Option<String>,
    stroke_selection: Selection,
    color_palette_open: bool,
    color_hex: String,
    export: ExportHandler,
    error: Option<Error>,
    edits: Vec<Edit>,
    undo_stack: Vec<Change>,
    redo_stack: Vec<Change>,
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
            tool: settings.tool,
            mode: Mode::Draw,
            initial_settings: settings,
            sizes: settings.into(),
            size_input: None,
            stroke_selection: settings.stroke_color.into(),
            color_palette_open: false,
            color_hex: format_hex(settings.stroke_color),
            export,
            error: None,
            edits: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
            Message::SelectTool(tool) => self.select_tool(tool),
            Message::SelectionMode => self.enter_selection_mode(),
            Message::SelectEdit(index) => self.select_edit(index),
            Message::DeleteSelected => self.delete_selected(),
            Message::SelectStrokeColor(stroke_color) => {
                self.select_stroke_color(stroke_color);
                return Task::none();
            }
            Message::Size(message) => {
                self.update_size(message);
                return Task::none();
            }
            Message::ToggleColorPalette => {
                self.toggle_color_pallete();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Settings, ThemePreference, Tool,
        annotation::AnnotationDraft,
        editor::{Message, SizeMessage},
        render::render_image,
        size::{SizeKind, Sizes},
        style::RED,
        viewport::Viewport,
    };
    use claims::{assert_ok, assert_some};
    use iced::{Color, Point, Rectangle, Size};
    use rstest::rstest;
    use std::rc::Rc;

    const SETTINGS: Settings = Settings {
        theme: ThemePreference::Dark,
        tool: Tool::Arrow,
        stroke_color: RED,
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

    fn stripes() -> Vec<u8> {
        (0..512)
            .flat_map(|index| {
                if index % 2 == 0 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect()
    }

    fn striped_editor() -> Editor {
        Editor::new(
            Handle::from_rgba(32, 16, stripes()),
            Size::new(32.0, 16.0),
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
            stroke_color: RED,
            size: Sizes::from(SETTINGS).get(tool.size_kind()),
            points: vec![
                bounds.position(),
                Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
            ],
        })
    }

    fn delete(editor: &mut Editor, index: usize) {
        for message in [
            Message::SelectionMode,
            Message::SelectEdit(Some(index)),
            Message::DeleteSelected,
        ] {
            let _ = editor.update(message);
        }
    }

    fn assert_figures(figures: &[Edit], expected: &[(Tool, Rectangle)]) {
        let viewport = assert_some!(Viewport::new(
            Size::new(100.0, 100.0),
            Rectangle::with_size(Size::new(100.0, 100.0))
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
        assert!(editor.redo_stack.is_empty());
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
        assert!(editor.redo_stack.is_empty());
    }

    #[test]
    fn exhausted_history_is_a_noop() {
        let mut editor = editor();

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[]);
        assert!(editor.redo_stack.is_empty());

        let figure = (Tool::Rectangle, bounds(10.0));
        let _ = editor.update(add_figure(figure.0, figure.1));

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Undo);
        assert_figures(&editor.edits, &[]);
        assert_eq!(editor.redo_stack.len(), 1);

        let _ = editor.update(Message::Redo);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[figure]);
        assert!(editor.redo_stack.is_empty());
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

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[figure]);
        assert!(editor.redo_stack.is_empty());
    }

    #[test]
    fn deletion_undoes_at_its_original_position_and_redoes() {
        let mut editor = editor();
        let figures = [
            (Tool::Rectangle, bounds(10.0)),
            (Tool::Arrow, bounds(20.0)),
            (Tool::Ellipse, bounds(30.0)),
        ];
        let [first, second, third] = figures;

        for (tool, bounds) in figures {
            let _ = editor.update(add_figure(tool, bounds));
        }

        delete(&mut editor, 1);
        assert_figures(&editor.edits, &[first, third]);
        assert_eq!(editor.mode, Mode::Select(None));

        let _ = editor.update(Message::Undo);
        assert_figures(&editor.edits, &figures);

        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[first, third]);

        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Undo);
        assert_figures(&editor.edits, &[first, second]);

        delete(&mut editor, 0);
        let _ = editor.update(Message::Redo);
        assert_figures(&editor.edits, &[second]);
        assert!(editor.redo_stack.is_empty());
    }

    #[test]
    fn deleting_an_edit_rerenders_the_rest_from_the_original_in_order() {
        let region = Rectangle::new(Point::ORIGIN, Size::new(16.0, 16.0));
        let line = Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            stroke_color: Color::WHITE,
            size: 4,
            points: vec![Point::new(0.0, 8.0), Point::new(32.0, 8.0)],
        });
        let mut expected = striped_editor();
        let _ = expected.update(add_figure(Tool::Pixelate, region));
        let _ = expected.update(add_figure(Tool::Blur, region));
        let expected = assert_ok!(expected.render_image());

        let mut editor = striped_editor();
        let _ = editor.update(add_figure(Tool::Pixelate, region));
        let _ = editor.update(line);
        let _ = editor.update(add_figure(Tool::Blur, region));
        let edited = assert_ok!(editor.render_image());

        delete(&mut editor, 1);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), expected.rgba());

        let _ = editor.update(Message::Undo);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), edited.rgba());
    }

    #[test]
    fn appearance_changes_preserve_existing_figures_and_history() {
        let mut editor = editor();
        let blue = Color::from_rgb8(64, 128, 255);
        let green = Color::from_rgb8(64, 200, 96);

        let thin = Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            stroke_color: RED,
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
        let _ = editor.update(Message::Size(SizeMessage::Select(SizeKind::StrokeWidth, 8)));
        let _ = editor.update(Message::SelectStrokeColor(blue));
        let _ = editor.update(thick);
        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Size(SizeMessage::Select(SizeKind::StrokeWidth, 4)));
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

        assert_eq!(appearances, [(2, RED), (8, blue)]);

        assert_eq!(editor.sizes.get(SizeKind::StrokeWidth), 4);
        assert!(editor.redo_stack.is_empty());
        assert_eq!(
            Color::from(editor.stroke_selection).into_rgba8(),
            green.into_rgba8()
        );
    }

    #[rstest]
    #[case(Tool::Pixelate)]
    #[case(Tool::Blur)]
    fn pixel_effect_history_restores_pixels_and_preserves_the_original(#[case] tool: Tool) {
        let mut editor = striped_editor();
        let pixels = stripes();
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
        assert_eq!(editor.redo_stack.len(), 1);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), pixels);

        let _ = editor.update(Message::Redo);
        assert_eq!(assert_ok!(editor.render_image()).rgba(), processed.rgba());
    }
}
