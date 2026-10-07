use std::fmt::Display;

use crate::figure::{EllipseFigure, Figure, RectangleFigure};
use iced::Rectangle;

/// The figure created by the next drag
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Rectangle,
    Ellipse,
}

impl Tool {
    pub fn figure(self, bounds: Rectangle) -> Box<dyn Figure> {
        match self {
            Self::Rectangle => Box::new(RectangleFigure::new(bounds)),
            Self::Ellipse => Box::new(EllipseFigure::new(bounds)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
        }
    }
}

impl Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
