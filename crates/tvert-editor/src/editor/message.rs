use crate::{annotation::AnnotationDraft, export::ExportAction, size::SizeKind, tool::Tool};
use iced::Color;
use iced_color_triangle::Selection;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddEdit(AnnotationDraft),
    SelectTool(Tool),
    SelectSize(SizeKind, u16),
    ToggleColorPalette,
    CloseColorPalette,
    SelectStrokeColor(Color),
    SelectStrokeSelection(Selection),
    EditColorHex(String),
    Undo,
    Redo,
    Export(ExportAction),
}
