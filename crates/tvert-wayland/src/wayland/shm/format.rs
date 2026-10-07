use wayland_client::{WEnum, protocol::wl_shm::Format};

use crate::Error;

/// Four-byte-per-pixel SHM formats supported by Tvert
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmFormat {
    /// ARGB8888 with premultiplied alpha
    Argb8888,
    /// XRGB8888 with opaque pixels
    Xrgb8888,
    /// ABGR8888 with premultiplied alpha
    Abgr8888,
    /// XBGR8888 with opaque pixels
    Xbgr8888,
}
impl TryFrom<WEnum<Format>> for ShmFormat {
    type Error = Error;

    fn try_from(format: WEnum<Format>) -> Result<Self, Self::Error> {
        match format {
            WEnum::Value(Format::Argb8888) => Ok(Self::Argb8888),
            WEnum::Value(Format::Xrgb8888) => Ok(Self::Xrgb8888),
            WEnum::Value(Format::Abgr8888) => Ok(Self::Abgr8888),
            WEnum::Value(Format::Xbgr8888) => Ok(Self::Xbgr8888),
            other => Err(Error::UnsupportedShmFormat(other)),
        }
    }
}

impl From<ShmFormat> for Format {
    fn from(format: ShmFormat) -> Self {
        match format {
            ShmFormat::Argb8888 => Self::Argb8888,
            ShmFormat::Xrgb8888 => Self::Xrgb8888,
            ShmFormat::Abgr8888 => Self::Abgr8888,
            ShmFormat::Xbgr8888 => Self::Xbgr8888,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use rstest::rstest;
    use std::assert_matches;

    #[rstest]
    #[case(Format::Argb8888)]
    #[case(Format::Xrgb8888)]
    #[case(Format::Abgr8888)]
    #[case(Format::Xbgr8888)]
    fn accepted_formats_retain_their_protocol_value(#[case] original: Format) {
        let format = assert_ok!(ShmFormat::try_from(WEnum::Value(original)));

        assert_eq!(Format::from(format), original);
    }

    #[rstest]
    #[case(WEnum::Value(Format::Rgb565))]
    #[case(WEnum::Unknown(u32::MAX))]
    fn rejected_formats_retain_the_original_value(#[case] original: WEnum<Format>) {
        assert_matches!(
            assert_err!(ShmFormat::try_from(original)),
            Error::UnsupportedShmFormat(actual) if actual == original
        );
    }
}
