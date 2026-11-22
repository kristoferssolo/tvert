use crate::{
    annotation::AnnotationDraft, crop::Crop, export::ExportAction, size::SizeKind, tool::Tool,
    viewport::Navigation,
};
use iced::{Color, Point, Vector, widget::text_editor};
use iced_color_triangle::Selection;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddEdit(AnnotationDraft),
    SelectTool(Tool),
    SelectionMode,
    SelectEdit(Option<usize>),
    DeleteSelected,
    CropMode,
    /// Proposes a crop while in crop mode
    Crop(Crop),
    /// Records the proposed crop as one history entry and leaves crop mode
    ApplyCrop,
    /// Clears the applied crop as one history entry, along with any proposal
    ResetCrop,
    Move(MoveMessage),
    Text(TextMessage),
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

/// Typing a text annotation in place
#[derive(Debug, Clone, PartialEq)]
pub enum TextMessage {
    /// A click with the text tool, at its image position when over the image.
    /// Records text being typed, or otherwise starts typing there.
    Place(Option<Point>),
    Edit(text_editor::Action),
    /// Records the typed text as one history entry, discarding empty text
    Commit,
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
