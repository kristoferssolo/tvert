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
