mod color;
mod document;
mod message;
mod movement;
mod selection;
pub mod shortcuts;
mod size;
mod view;

pub use message::{Message, MoveMessage, SizeMessage};

use crate::{
    Error, Settings,
    color::format_hex,
    edit::Edit,
    editor::{
        document::{ApplyError, Change, Document},
        movement::Movement,
    },
    export::ExportAction,
    mode::Mode,
    render::render_image,
    size::Sizes,
    tool::Tool,
    viewport::Navigation,
};
#[cfg(test)]
use crate::{ThemePreference, export::ExportBindings, style::RED};
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
    document: Document,
    image_size: Size,
    navigation: Navigation,
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
    movement: Option<Movement>,
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
            document: Document::new(preview),
            image_size,
            navigation: Navigation::Fit,
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
            movement: None,
            cache: Cache::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if !matches!(message, Message::Move(_)) {
            self.cancel_move();
        }
        match message {
            Message::AddEdit(draft) => {
                let Ok(edit) = Edit::try_from(draft) else {
                    return Task::none();
                };
                self.update_document(|document| {
                    document.record(Change::Insert(document.edits().len(), edit))
                });
            }
            Message::SelectTool(tool) => self.select_tool(tool),
            Message::SelectionMode => self.enter_selection_mode(),
            Message::SelectEdit(index) => self.select_edit(index),
            Message::DeleteSelected => {
                if let Mode::Select(Some(index)) = self.mode {
                    self.update_document(|document| document.record(Change::Remove(index)));
                }
            }
            Message::Move(message) => self.update_move(message),
            Message::Navigate(navigation) => {
                self.navigation = navigation;
                return Task::none();
            }
            Message::SelectStrokeColor(stroke_color) => {
                self.select_stroke_color(stroke_color);
                return Task::none();
            }
            Message::Size(message) => {
                self.update_size(message);
                return Task::none();
            }
            Message::ToggleColorPalette => {
                self.toggle_color_palette();
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
            Message::Undo => {
                self.update_document(Document::undo);
            }
            Message::Redo => {
                self.update_document(Document::redo);
            }
            Message::Export(action) => return self.export(action),
        }
        self.cache.clear();
        Task::none()
    }

    /// Returns the committed image at full resolution.
    fn render_image(&self) -> Result<CapturedImage, Error> {
        render_image(self.document.preview(), &[])
    }

    fn export(&mut self, action: ExportAction) -> Task<Message> {
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

    /// Discards unapplied size text and closes the color palette.
    fn dismiss_controls(&mut self) {
        self.size_input = None;
        self.close_color_palette();
    }

    /// Runs a document operation, shows its result, and returns whether it applied.
    ///
    /// Inserting or removing an edit clears the selection because indices may
    /// shift; replacing preserves it.
    fn update_document(
        &mut self,
        operation: impl FnOnce(&mut Document) -> Result<(), ApplyError>,
    ) -> bool {
        let count = self.document.edits().len();
        match operation(&mut self.document) {
            Ok(()) => {
                self.error = None;
                if self.document.edits().len() != count {
                    self.select_edit(None);
                }
                true
            }
            Err(ApplyError::Rejected) => false,
            Err(ApplyError::Render(error)) => {
                self.error = Some(error);
                false
            }
        }
    }
}

#[cfg(test)]
impl Editor {
    /// An editor over an RGBA image with fixed settings and exports that succeed
    fn for_test(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self::new(
            Handle::from_rgba(width, height, rgba),
            Size::from((width, height)),
            Settings {
                theme: ThemePreference::Dark,
                tool: Tool::Arrow,
                stroke_color: RED,
                stroke_width: 4,
                pixelation_block_size: 16,
                blur_sigma: 8,
                export_bindings: ExportBindings::parse("Ctrl+C", "Ctrl+S", "Ctrl+Shift+C"),
            },
            Rc::new(|_, _| Ok(())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotation::AnnotationDraft, size::SizeKind, style::RED};
    use claims::{assert_ok, assert_some};
    use iced::{Color, Point};

    fn editor() -> Editor {
        Editor::for_test(32, 16, [20, 40, 60, 255].repeat(512))
    }

    fn line(size: u16, stroke_color: Color) -> Message {
        Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            size,
            stroke_color,
            points: vec![Point::new(2.0, 8.0), Point::new(30.0, 8.0)],
        })
    }

    #[test]
    fn navigation_leaves_the_committed_image_exports_and_history_alone() {
        let mut editor = editor();
        let _ = editor.update(line(2, RED));
        let preview = editor.document.preview().id();
        let exported = assert_ok!(editor.render_image());

        for navigation in [
            Navigation::Manual {
                scale: 8.0,
                center: Point::new(4.0, 12.0),
            },
            Navigation::Fit,
        ] {
            let _ = editor.update(Message::Navigate(navigation));
            let image = assert_ok!(editor.render_image());

            assert_eq!(editor.document.preview().id(), preview);
            assert_eq!(image.size(), exported.size());
            assert_eq!(image.rgba(), exported.rgba());
            assert!(editor.document.can_undo());
        }
    }

    #[test]
    fn appearance_changes_preserve_existing_figures_and_history() {
        let mut editor = editor();
        let blue = Color::from_rgb8(64, 128, 255);
        let green = Color::from_rgb8(64, 200, 96);

        let _ = editor.update(line(2, RED));
        let _ = editor.update(Message::Size(SizeMessage::Select(SizeKind::StrokeWidth, 8)));
        let _ = editor.update(Message::SelectStrokeColor(blue));
        let _ = editor.update(line(8, blue));
        let _ = editor.update(Message::Undo);
        let _ = editor.update(Message::Size(SizeMessage::Select(SizeKind::StrokeWidth, 4)));
        let _ = editor.update(Message::SelectStrokeColor(green));
        let _ = editor.update(Message::Redo);

        let appearances = editor
            .document
            .edits()
            .iter()
            .map(|edit| {
                let annotation = assert_some!(edit.annotation());
                (annotation.stroke_width, annotation.stroke_color)
            })
            .collect::<Vec<_>>();

        assert_eq!(appearances, [(2, RED), (8, blue)]);

        assert_eq!(editor.sizes.get(SizeKind::StrokeWidth), 4);
        assert!(!editor.document.can_redo());
        assert_eq!(
            Color::from(editor.stroke_selection).into_rgba8(),
            green.into_rgba8()
        );
    }

    #[test]
    fn a_failed_edit_shows_its_error_until_the_next_change() {
        let mut editor = editor();

        let _ = editor.update(Message::AddEdit(AnnotationDraft {
            tool: Tool::Pixelate,
            size: 4,
            stroke_color: RED,
            points: vec![Point::new(40.0, 0.0), Point::new(48.0, 8.0)],
        }));
        assert!(editor.error.is_some());
        assert!(editor.document.edits().is_empty());

        let _ = editor.update(line(2, RED));
        assert!(editor.error.is_none());
        assert_eq!(editor.document.edits().len(), 1);
    }
}
