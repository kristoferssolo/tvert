use crate::{FrozenOutput, wayland::DesktopPosition};

/// Frozen outputs and a selected rectangle in logical desktop coordinates.
///
/// Captures retain buffer orientation. The bounds preserve fractional edges and
/// may include gaps or extend beyond the outputs. Composition and pixel rounding
/// belong to the caller. Selector feedback is never painted into these captures.
#[derive(Debug)]
pub struct RegionSelection {
    pub outputs: Vec<FrozenOutput>,
    pub bounds: RegionBounds,
}

/// A nonempty logical desktop rectangle with finite, normalized edges
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionBounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl RegionBounds {
    pub(crate) fn from_positions(start: DesktopPosition, end: DesktopPosition) -> Option<Self> {
        let left = start.x().min(end.x());
        let top = start.y().min(end.y());
        let right = start.x().max(end.x());
        let bottom = start.y().max(end.y());
        if left >= right || top >= bottom {
            return None;
        }
        Some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    #[must_use]
    pub const fn left(self) -> f64 {
        self.left
    }

    #[must_use]
    pub const fn top(self) -> f64 {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> f64 {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> f64 {
        self.bottom
    }
}
