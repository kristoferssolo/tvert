use crate::Error;
use std::{fmt::Display, num::NonZeroI32};

/// Suggested dimensions in surface-local coordinates.
///
/// A zero dimension means the client must choose that dimension.
#[derive(Debug, Clone, Copy)]
pub struct SuggestedSurfaceSize {
    pub width: u32,
    pub height: u32,
}

/// Positive surface dimensions representable as viewport destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    width: NonZeroI32,
    height: NonZeroI32,
}

impl SurfaceSize {
    pub const fn width(self) -> i32 {
        self.width.get()
    }

    pub const fn height(self) -> i32 {
        self.height.get()
    }
}

impl TryFrom<SuggestedSurfaceSize> for SurfaceSize {
    type Error = Error;

    fn try_from(size: SuggestedSurfaceSize) -> Result<Self, Self::Error> {
        let unsupported = || Error::UnsupportedOverlaySize {
            width: size.width,
            height: size.height,
        };

        let width = i32::try_from(size.width)
            .ok()
            .and_then(NonZeroI32::new)
            .ok_or_else(unsupported)?;
        let height = i32::try_from(size.height)
            .ok()
            .and_then(NonZeroI32::new)
            .ok_or_else(unsupported)?;

        Ok(Self { width, height })
    }
}

impl Display for SuggestedSurfaceSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_err, assert_matches, assert_ok};
    use rstest::rstest;

    use super::*;

    #[test]
    fn accepts_the_maximum_viewport_dimensions() {
        let size = assert_ok!(SurfaceSize::try_from(SuggestedSurfaceSize {
            width: 2_147_483_647,
            height: 2_147_483_647,
        }));
        assert_eq!(size.width(), i32::MAX);
        assert_eq!(size.height(), i32::MAX);
    }

    #[rstest]
    #[case(0, 1)]
    #[case(1, 0)]
    #[case(2_147_483_648, 1)]
    #[case(1, 2_147_483_648)]
    fn rejects_unresolved_or_unpresentable_dimensions(#[case] width: u32, #[case] height: u32) {
        assert_matches!(
            assert_err!(SurfaceSize::try_from(SuggestedSurfaceSize {
                width,
                height
            })),
            Error::UnsupportedOverlaySize { .. }
        );
    }
}
