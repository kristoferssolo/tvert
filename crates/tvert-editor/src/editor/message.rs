use crate::{
    annotation::AnnotationDraft, export::ExportAction, size::SizeKind, tool::Tool,
    viewport::Navigation,
};
use iced::{Color, Vector};
use iced_color_triangle::Selection;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddEdit(AnnotationDraft),
    SelectTool(Tool),
    SelectionMode,
    SelectEdit(Option<usize>),
    DeleteSelected,
    Move(MoveMessage),
    Navigate(Navigation),
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

/// Dragging the selected edit by offsets in captured-image pixels
/// from where the gesture began
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveMessage {
    /// Previews the edit at the offset
    Drag(Vector),
    /// Records the edit at the offset as one history entry
    Finish(Vector),
    /// Restores the edit to where the gesture began
    Cancel,
}
