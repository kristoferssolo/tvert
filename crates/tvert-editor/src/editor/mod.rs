pub mod color;
mod history;
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
    editor::{history::Change, movement::Movement},
    export::ExportAction,
    mode::Mode,
    render::render_image,
    size::Sizes,
    tool::Tool,
    viewport::Navigation,
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
    edits: Vec<Edit>,
    undo_stack: Vec<Change>,
    redo_stack: Vec<Change>,
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
            original: preview.clone(),
            preview,
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
            edits: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
                self.add_edit(edit);
            }
            Message::SelectTool(tool) => self.select_tool(tool),
            Message::SelectionMode => self.enter_selection_mode(),
            Message::SelectEdit(index) => self.select_edit(index),
            Message::DeleteSelected => self.delete_selected(),
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
    use crate::{ThemePreference, annotation::AnnotationDraft, style::RED};
    use claims::assert_ok;
    use iced::Point;

    #[test]
    fn navigation_leaves_the_committed_image_exports_and_history_alone() {
        let mut editor = Editor::new(
            Handle::from_rgba(32, 16, [20, 40, 60, 255].repeat(512)),
            Size::new(32.0, 16.0),
            Settings {
                theme: ThemePreference::Dark,
                tool: Tool::Arrow,
                stroke_color: RED,
                stroke_width: 4,
                pixelation_block_size: 4,
                blur_sigma: 8,
            },
            Rc::new(|_, _| Ok(())),
        );
        let _ = editor.update(Message::AddEdit(AnnotationDraft {
            tool: Tool::Line,
            size: 2,
            stroke_color: RED,
            points: vec![Point::new(2.0, 8.0), Point::new(30.0, 8.0)],
        }));
        let preview = editor.preview.id();
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

            assert_eq!(editor.preview.id(), preview);
            assert_eq!(image.size(), exported.size());
            assert_eq!(image.rgba(), exported.rgba());
            assert_eq!(editor.undo_stack.len(), 1);
        }
    }
}
