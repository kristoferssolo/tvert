//! A hue ring and triangle color selector for Iced.
//!
//! Retain [`Selection`] in application state and replace it on each change.
//! This preserves the chosen hue when the color becomes black, white, or gray.

mod selection;
mod wheel;

pub use selection::Selection;

use iced_core::Element;
use iced_widget::graphics::geometry::Renderer;

/// Creates a 240 × 240 logical-pixel hue ring and triangle selector.
///
/// The callback receives candidate selections during dragging and on release.
/// The caller decides when to apply the color, using [`iced_core::Color::from`].
/// Dragging beyond a triangle edge clamps the selection to that edge.
/// The triangle rotates so its colored corner points toward the hue-ring handle.
/// The widget preserves alpha and works with any Iced canvas renderer and theme.
pub fn color_triangle<'a, Message: 'a, Theme: 'a, R: Renderer + 'static>(
    selection: Selection,
    on_change: impl Fn(Selection) -> Message + 'a,
) -> Element<'a, Message, Theme, R> {
    wheel::view(selection, on_change)
}
