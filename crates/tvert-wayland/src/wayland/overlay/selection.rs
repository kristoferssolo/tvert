use crate::wayland::overlay::{SurfacePosition, SurfaceRect, size::SurfaceSize};

/// Selection progress with endpoints in surface-local coordinates.
///
/// Completed endpoints have not yet been normalized or clipped.
#[derive(Debug, Clone, Copy, Default)]
pub enum SelectionState {
    #[default]
    Idle,
    /// The left button is held while the endpoint moves
    Dragging {
        start: SurfacePosition,
        current: SurfacePosition,
    },
    /// The left button was released
    Complete {
        start: SurfacePosition,
        end: SurfacePosition,
    },
}

impl SelectionState {
    /// Starts a drag, replacing the previous selection
    pub const fn begin(&mut self, position: SurfacePosition) {
        *self = Self::Dragging {
            start: position,
            current: position,
        };
    }

    /// Updates an active drag's endpoint
    pub const fn update(&mut self, position: SurfacePosition) {
        let Self::Dragging { current, .. } = self else {
            return;
        };

        *current = position;
    }

    /// Completes an active drag using its latest endpoint
    pub const fn finish(&mut self) {
        let Self::Dragging { start, current } = *self else {
            return;
        };

        *self = Self::Complete {
            start,
            end: current,
        };
    }

    /// Discards an unfinished drag, preserving any completed selection
    pub const fn cancel_drag(&mut self) {
        if !matches!(self, Self::Dragging { .. }) {
            return;
        }

        *self = Self::Idle;
    }

    /// Returns the current nonempty selection clipped to the supplied bounds.
    ///
    /// Returns `None` for an idle selection or an empty clipped rectangle.
    pub fn rectangle(self, bounds: SurfaceSize) -> Option<SurfaceRect> {
        let (start, end) = match self {
            Self::Idle => return None,
            Self::Dragging {
                start,
                current: end,
            }
            | Self::Complete { start, end } => (start, end),
        };

        SurfaceRect::from_position(start, end, bounds)
    }
}
