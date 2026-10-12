/// How a primary press on the image is interpreted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Drag to create an edit with the current tool
    Draw,
    /// Click to pick an existing edit, by its index in rendering order
    Select(Option<usize>),
}
