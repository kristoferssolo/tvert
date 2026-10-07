use crate::{stroke_width::StrokeWidth, tool::Tool};
use iced::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddFigure {
        tool: Tool,
        stroke_width: StrokeWidth,
        start: Point,
        end: Point,
    },
    SelectTool(Tool),
    SelectStrokeWidth(StrokeWidth),
    Undo,
    Redo,
}
