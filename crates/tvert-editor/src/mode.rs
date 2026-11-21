use crate::crop::Crop;

/// How a primary press on the image is interpreted
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// Drag to create an edit with the current tool
    Draw,
    /// Click to pick an existing edit, by its index in rendering order
    Select(Option<usize>),
    /// Drag over the whole image to propose a crop, which Enter applies
    Crop(Option<Crop>),
}
