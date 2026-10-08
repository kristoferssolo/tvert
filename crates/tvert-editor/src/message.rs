use crate::{
    annotation::AnnotationDraft, export::ExportAction, stroke_width::StrokeWidth, tool::Tool,
};
use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddAnnotation(AnnotationDraft),
    SelectTool(Tool),
    SelectStrokeWidth(StrokeWidth),
    SelectStrokeColor(Color),
    ToggleColorPallete,
    CloseColorPallete,
    Undo,
    Redo,
    Export(ExportAction),
}
