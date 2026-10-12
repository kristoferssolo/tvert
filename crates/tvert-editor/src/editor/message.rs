use crate::{annotation::AnnotationDraft, export::ExportAction, size::SizeKind, tool::Tool};
use iced::Color;
use iced_color_triangle::Selection;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddEdit(AnnotationDraft),
    SelectTool(Tool),
    Size(SizeMessage),
    ToggleColorPalette,
    CloseColorPalette,
    SelectStrokeColor(Color),
    SelectStrokeSelection(Selection),
    EditColorHex(String),
    Undo,
    Redo,
    Export(ExportAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeMessage {
    /// A slider value, including Ctrl+click resets to the configured default
    Select(SizeKind, u16),
    /// Typed text, applied only when it parses within the kind's range
    Edit(SizeKind, String),
    /// Enter in the size field, which discards any unapplied text
    Submit,
}
