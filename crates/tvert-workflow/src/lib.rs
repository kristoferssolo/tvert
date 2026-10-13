mod capture;
mod clipboard;
mod destination;
mod error;
mod export;
mod pattern;
mod target;

pub use capture::capture;
pub use clipboard::{ClipboardCopy, prepare_png_copy};
pub use destination::Destination;
pub use error::{EmptyOutputName, Error, PatternError};
pub use export::save_png;
pub use pattern::PathPattern;
pub use target::{OutputSelection, Target};
