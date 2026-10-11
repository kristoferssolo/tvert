use crate::{
    Error,
    style::{
        BLOCK_SIZE_RANGE, BLUR_SIGMA_RANGE, DEFAULT_BLOCK_SIZE, DEFAULT_BLUR_SIGMA,
        DEFAULT_STROKE_WIDTH, STROKE_WIDTH_RANGE,
    },
};

/// Initial drawing settings for an editor session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Stroke width in image pixels, from 1 to 32
    pub stroke_width: u16,
    /// Pixelation block size in image pixels, from 4 to 64
    pub pixelation_block_size: u16,
    /// Blur sigma in image pixels, from 4 to 64
    pub blur_sigma: u16,
}

impl Settings {
    /// Checks that each setting is within its supported slider range.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSetting`] for the first unsupported value.
    pub fn validate(self) -> Result<(), Error> {
        for (name, value, range) in [
            ("stroke_width", self.stroke_width, STROKE_WIDTH_RANGE),
            (
                "pixelation_block_size",
                self.pixelation_block_size,
                BLOCK_SIZE_RANGE,
            ),
            ("blur_sigma", self.blur_sigma, BLUR_SIGMA_RANGE),
        ] {
            if !range.contains(&value) {
                return Err(Error::InvalidSetting {
                    name,
                    value,
                    min: *range.start(),
                    max: *range.end(),
                });
            }
        }

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            stroke_width: DEFAULT_STROKE_WIDTH,
            pixelation_block_size: DEFAULT_BLOCK_SIZE,
            blur_sigma: DEFAULT_BLUR_SIGMA,
        }
    }
}
