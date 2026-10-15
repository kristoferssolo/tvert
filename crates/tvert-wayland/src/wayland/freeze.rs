use crate::{Error, FrozenOutput, Output, OutputId, Wayland};
use tvert_image::{CapturedImage, ImageTransform};

trait Source {
    fn snapshot(&self) -> Result<Vec<(Output, ImageTransform)>, Error>;
    fn capture(&mut self, output: OutputId) -> Result<CapturedImage, Error>;
}

impl Source for Wayland {
    fn snapshot(&self) -> Result<Vec<(Output, ImageTransform)>, Error> {
        self.outputs()
            .map(|output| {
                let state = self
                    .state
                    .output_states
                    .get(&output.id)
                    .ok_or(Error::UnknownOutput(output.id))?;
                Ok((output.clone(), state.displayed_transform()?))
            })
            .collect()
    }

    fn capture(&mut self, output: OutputId) -> Result<CapturedImage, Error> {
        self.capture_buffer(output)
    }
}

impl Wayland {
    /// Captures all known outputs in registry-id order.
    ///
    /// Captures are sequential, exclude the cursor, and retain buffer orientation.
    /// No selector surfaces are created. Call this before mapping selectors.
    /// Returns no partial batch if capture fails or metadata changes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoOutputs`] for an empty output list,
    /// [`Error::OutputsChanged`] if placement or orientation changes during
    /// capture, or any error from [`Self::capture_output`].
    pub fn freeze_outputs(&mut self) -> Result<Vec<FrozenOutput>, Error> {
        freeze_from(self)
    }

    pub(super) fn validate_frozen_ouputs<'a>(
        &self,
        outputs: impl IntoIterator<Item = &'a FrozenOutput>,
    ) -> Result<(), Error> {
        let mut count = 0usize;
        for frozen in outputs {
            count = count.saturating_add(1);
            let unchanged = self.state.outputs.get(&frozen.output.id) == Some(&frozen.output)
                && self
                    .state
                    .output_states
                    .get(&frozen.output.id)
                    .is_some_and(|output| {
                        output
                            .displayed_transform()
                            .is_ok_and(|orientation| orientation == frozen.orientation)
                    });
            if !unchanged {
                return Err(Error::OutputsChanged);
            }
        }
        if count != self.state.outputs.len() {
            return Err(Error::OutputsChanged);
        }
        Ok(())
    }
}

fn freeze_from(source: &mut impl Source) -> Result<Vec<FrozenOutput>, Error> {
    let mut before = source.snapshot()?;
    before.sort_unstable_by_key(|(output, _)| output.id.0);
    if before.is_empty() {
        return Err(Error::NoOutputs);
    }

    let frozen = before
        .iter()
        .map(|(output, orientation)| {
            Ok(FrozenOutput {
                output: output.clone(),
                image: source.capture(output.id)?,
                orientation: *orientation,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let mut after = source.snapshot()?;
    after.sort_unstable_by_key(|(output, _)| output.id.0);
    if before != after {
        return Err(Error::OutputsChanged);
    }
    Ok(frozen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhysicalSize;
    use claims::{assert_err, assert_ok};
    use rstest::rstest;
    use tvert_image::PixelSize;

    #[derive(Clone, Copy, Debug)]
    enum Failure {
        Capture,
        Remove,
        Scale,
        Rotate,
    }

    struct Fake {
        outputs: Vec<(Output, ImageTransform)>,
        calls: Vec<OutputId>,
        failure: Option<Failure>,
    }

    impl Fake {
        fn new(failure: Option<Failure>) -> Self {
            let output = |id| {
                (
                    Output {
                        id: OutputId(id),
                        name: format!("DP-{id}"),
                        size: PhysicalSize {
                            width: 2,
                            height: 1,
                        },
                        scale: 1,
                        logical: None,
                    },
                    ImageTransform::Normal,
                )
            };
            Self {
                outputs: vec![output(2), output(1)],
                calls: Vec::new(),
                failure,
            }
        }
    }

    impl Source for Fake {
        fn snapshot(&self) -> Result<Vec<(Output, ImageTransform)>, Error> {
            Ok(self.outputs.clone())
        }

        fn capture(&mut self, output: OutputId) -> Result<CapturedImage, Error> {
            self.calls.push(output);
            match self.failure {
                Some(Failure::Capture) if output == OutputId(2) => {
                    return Err(Error::ScreencopyFailed);
                }
                Some(Failure::Remove) => {
                    self.outputs.retain(|(output, _)| output.id != OutputId(1));
                }
                Some(Failure::Scale) => {
                    if let Some((output, _)) = self.outputs.first_mut() {
                        output.scale = 2;
                    }
                }
                Some(Failure::Rotate) => {
                    if let Some((_, orientation)) = self.outputs.first_mut() {
                        *orientation = ImageTransform::Rotate180;
                    }
                }
                None if output == OutputId(1) => {
                    self.outputs.reverse();
                }
                _ => {}
            }
            let size = PixelSize::try_from((2, 1))?;
            Ok(CapturedImage::new(
                size,
                vec![10, 20, 30, 255, 40, 50, 60, 128],
            )?)
        }
    }

    #[test]
    fn captures_in_id_order_even_when_snapshot_order_changes() {
        let mut source = Fake::new(None);
        let frozen = assert_ok!(freeze_from(&mut source));
        assert_eq!(source.calls, [OutputId(1), OutputId(2)]);
        assert_eq!(
            frozen
                .iter()
                .map(|capture| capture.output.id)
                .collect::<Vec<_>>(),
            source.calls
        );
        for capture in frozen {
            assert_eq!(capture.image.rgba(), [10, 20, 30, 255, 40, 50, 60, 128]);
        }
    }

    #[rstest]
    #[case(Failure::Capture)]
    #[case(Failure::Remove)]
    #[case(Failure::Scale)]
    #[case(Failure::Rotate)]
    fn never_returns_a_partial_or_stale_batch(#[case] failure: Failure) {
        let mut source = Fake::new(Some(failure));
        let error = assert_err!(freeze_from(&mut source));
        let expected = match failure {
            Failure::Capture => matches!(error, Error::ScreencopyFailed),
            _ => matches!(error, Error::OutputsChanged),
        };
        assert!(expected, "{failure:?}: {error:?}");
    }

    #[test]
    fn rejects_an_empty_snapshot_before_capturing() {
        let mut source = Fake::new(None);
        source.outputs.clear();
        assert!(matches!(freeze_from(&mut source), Err(Error::NoOutputs)));
        assert_eq!(source.calls, []);
    }
}
