use crate::{annotation::AnnotationDraft, export::ExportAction, tool::Tool};
use iced::Color;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddEdit(AnnotationDraft),
    SelectTool(Tool),
    SelectStrokeColor(Color),
    SelectStrokeWidth(u16),
    SelectBlockSize(u16),
    SelectBlurRadius(u16),
    ToggleColorPallete,
    CloseColorPallete,
    Undo,
    Redo,
    Export(ExportAction),
}
