use crate::tool::Tool;
use iced::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddFigure {
        tool: Tool,
        start: Point,
        end: Point,
    },
    SelectTool(Tool),
    Undo,
    Redo,
}
