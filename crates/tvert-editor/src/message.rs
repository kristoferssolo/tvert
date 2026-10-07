use crate::{annotation::AnnotationDraft, stroke_width::StrokeWidth, tool::Tool};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddAnnotation(AnnotationDraft),
    SelectTool(Tool),
    SelectStrokeWidth(StrokeWidth),
    Undo,
    Redo,
}
