use iced::Rectangle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    AddRectangle(Rectangle),
    Undo,
    Redo,
}
