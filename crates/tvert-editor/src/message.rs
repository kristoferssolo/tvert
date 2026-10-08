use crate::{annotation::AnnotationDraft, stroke_width::StrokeWidth, tool::Tool};
use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddAnnotation(AnnotationDraft),
    SelectTool(Tool),
    SelectStrokeWidth(StrokeWidth),
    SelectStrokeColor(Color),
    Undo,
    Redo,
    Save,
}
