use crate::tool::Tool;
use iced::Rectangle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddFigure { tool: Tool, bounds: Rectangle },
    SelectTool(Tool),
    Undo,
    Redo,
}
