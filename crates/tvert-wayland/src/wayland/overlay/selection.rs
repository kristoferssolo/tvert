use crate::{
    LogicalRect, OutputId,
    wayland::overlay::{DesktopPosition, SurfaceRect, size::SurfaceSize},
};

/// A left-button drag with endpoints in logical desktop coordinates.
///
/// `origin` is the output whose selector surface received the press.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gesture {
    origin: OutputId,
    start: DesktopPosition,
    end: DesktopPosition,
}

impl Gesture {
    pub const fn origin(self) -> OutputId {
        self.origin
    }

    /// Returns the nonempty part of this gesture covered by one selector surface.
    ///
    /// `area` is the surface's logical desktop area. Returns `None` if the
    /// gesture does not overlap the surface or has no area.
    pub fn rectangle(self, size: SurfaceSize, area: LogicalRect) -> Option<SurfaceRect> {
        let start = self.start.to_surface(size, area)?;
        let end = self.end.to_surface(size, area)?;
        SurfaceRect::from_position(start, end, size)
    }
}

/// Selection progress shared by every selector surface.
///
/// Endpoints keep one meaning whichever surface reports pointer motion, so
/// pointer-focus changes do not affect a drag.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SelectionState {
    #[default]
    Idle,
    /// The left button is held while the endpoint moves
    Dragging(Gesture),
    /// The left button was released and the gesture awaits the selector loop
    Complete(Gesture),
}

impl SelectionState {
    /// Starts a drag, replacing the previous selection
    pub const fn begin(&mut self, origin: OutputId, position: DesktopPosition) {
        *self = Self::Dragging(Gesture {
            origin,
            start: position,
            end: position,
        });
    }

    /// Updates an active drag's endpoint
    pub const fn update(&mut self, position: DesktopPosition) {
        let Self::Dragging(gesture) = self else {
            return;
        };

        gesture.end = position;
    }

    /// Completes an active drag using its latest endpoint
    pub const fn finish(&mut self) {
        let Self::Dragging(gesture) = *self else {
            return;
        };

        *self = Self::Complete(gesture);
    }

    /// Discards an unfinished drag, preserving any completed selection
    pub const fn cancel_drag(&mut self) {
        if !matches!(self, Self::Dragging { .. }) {
            return;
        }

        *self = Self::Idle;
    }

    /// Takes a completed gesture, leaving the selection idle
    pub const fn take_complete(&mut self) -> Option<Gesture> {
        let Self::Complete(gesture) = *self else {
            return None;
        };

        *self = Self::Idle;
        Some(gesture)
    }

    /// Returns the current nonempty selection clipped to the supplied bounds.
    ///
    /// Returns `None` for an idle selection or an empty clipped rectangle.
    pub fn rectangle(self, size: SurfaceSize, area: LogicalRect) -> Option<SurfaceRect> {
        match self {
            Self::Idle => None,
            Self::Dragging(gesture) | Self::Complete(gesture) => gesture.rectangle(size, area),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::overlay::{SuggestedSurfaceSize, SurfacePosition};
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use std::num::NonZeroU32;

    const LEFT: OutputId = OutputId(1);
    const RIGHT: OutputId = OutputId(2);

    fn size() -> SurfaceSize {
        assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 100,
            height: 80,
        }))
    }

    fn area(x: i32, y: i32) -> LogicalRect {
        LogicalRect {
            x,
            y,
            width: assert_some!(NonZeroU32::new(100)),
            height: assert_some!(NonZeroU32::new(80)),
        }
    }

    fn surface(x: f64, y: f64) -> SurfacePosition {
        assert_some!(SurfacePosition::new(x, y))
    }

    fn desktop(position: (f64, f64), area: LogicalRect) -> DesktopPosition {
        assert_some!(DesktopPosition::from_surface(
            surface(position.0, position.1),
            size(),
            area
        ))
    }

    fn rect(start: (f64, f64), end: (f64, f64)) -> SurfaceRect {
        assert_some!(SurfaceRect::from_position(
            surface(start.0, start.1),
            surface(end.0, end.1),
            size(),
        ))
    }

    // Desktop, with the left output raised above the right output:
    //
    //   (-100,-40)
    //       +--------+
    //       |  LEFT  |(0,0)
    //       |        +--------+
    //       +--------|  RIGHT |
    //                |        |
    //                +--------+
    #[test]
    fn a_drag_held_by_its_origin_surface_spans_neighboring_outputs() {
        let left = area(-100, -40);
        let right = area(0, 0);
        let mut selection = SelectionState::Idle;

        // Under the implicit grab, motion past the left surface's right edge
        // stays relative to the left surface.
        selection.begin(LEFT, desktop((50.0, 10.0), left));
        selection.update(desktop((150.0, 70.0), left));
        selection.finish();

        let gesture = assert_some!(selection.take_complete());
        assert_eq!(gesture.origin(), LEFT);
        assert_some_eq!(
            gesture.rectangle(size(), left),
            rect((50.0, 10.0), (100.0, 70.0))
        );
        assert_some_eq!(
            gesture.rectangle(size(), right),
            rect((0.0, 0.0), (50.0, 30.0))
        );
        assert_none!(gesture.rectangle(size(), area(200, 0)));
    }

    #[test]
    fn a_click_completes_without_a_region() {
        let mut selection = SelectionState::Idle;
        selection.begin(RIGHT, desktop((20.0, 30.0), area(0, 0)));
        selection.finish();

        let gesture = assert_some!(selection.take_complete());
        assert_eq!(gesture.origin(), RIGHT);
        assert_none!(gesture.rectangle(size(), area(0, 0)));
        assert_eq!(selection, SelectionState::Idle);
    }

    #[test]
    fn cancelling_discards_only_an_unfinished_drag() {
        let start = desktop((10.0, 10.0), area(0, 0));
        let end = desktop((40.0, 50.0), area(0, 0));
        let mut selection = SelectionState::Idle;

        selection.begin(LEFT, start);
        selection.update(end);
        selection.cancel_drag();
        selection.finish();
        assert_eq!(selection, SelectionState::Idle);

        selection.begin(LEFT, start);
        selection.update(end);
        selection.finish();
        selection.update(start);
        selection.cancel_drag();
        let gesture = assert_some!(selection.take_complete());
        assert_some_eq!(
            gesture.rectangle(size(), area(0, 0)),
            rect((10.0, 10.0), (40.0, 50.0))
        );
    }
}
