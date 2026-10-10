use crate::{
    annotation::AnnotationDraft,
    export::ExportAction,
    style::{BlockSize, StrokeWidth},
    tool::Tool,
};
use iced::Color;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddEdit(AnnotationDraft),
    SelectTool(Tool),
    SelectStrokeWidth(StrokeWidth),
    SelectStrokeColor(Color),
    ToggleColorPallete,
    CloseColorPallete,
    Undo,
    Redo,
    Export(ExportAction),
    SelectBlockSize(BlockSize),
}
