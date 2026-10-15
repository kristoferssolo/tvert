use crate::{
    Error, OutputSelection,
    desktop::{Screen, compose},
    target::Target,
};
use tvert_image::CapturedImage;
use tvert_wayland::{Output, OutputId, Wayland};

/// The compositor operations capture needs, separated for testing
trait Backend {
    fn outputs(&self) -> impl Iterator<Item = &Output>;
    fn capture_output(&mut self, output: OutputId) -> Result<CapturedImage, tvert_wayland::Error>;
    fn select_screen(&mut self) -> Result<Option<CapturedImage>, tvert_wayland::Error>;
    fn select_region(
        &mut self,
        output: OutputId,
    ) -> Result<Option<CapturedImage>, tvert_wayland::Error>;
}

impl Backend for Wayland {
    fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.outputs()
    }

    fn capture_output(&mut self, output: OutputId) -> Result<CapturedImage, tvert_wayland::Error> {
        self.capture_output(output)
    }

    fn select_screen(&mut self) -> Result<Option<CapturedImage>, tvert_wayland::Error> {
        self.select_screen()
    }

    fn select_region(
        &mut self,
        output: OutputId,
    ) -> Result<Option<CapturedImage>, tvert_wayland::Error> {
        self.select_region(output)
    }
}

/// Connects to the compositor and captures `target` into an image.
///
/// Screen and region capture can wait for selection. Returns `None` if the user
/// cancels. Never touches the filesystem, and closes the connection before
/// returning so the image can be handed to the editor.
///
/// # Errors
///
/// Returns any error from [`crate::OutputSelection::resolve`], or
/// [`Error::Capture`] if connecting, capturing, or selecting fails.
pub fn capture(target: &Target) -> Result<Option<CapturedImage>, Error> {
    let mut wayland = Wayland::connect()?;
    capture_from(&mut wayland, target)
}

fn capture_from(
    backend: &mut impl Backend,
    target: &Target,
) -> Result<Option<CapturedImage>, Error> {
    match target {
        Target::Desktop => capture_desktop(backend).map(Some),
        Target::Screen(OutputSelection::All) => Ok(backend.select_screen()?),
        Target::Screen(output) => {
            let output = output.resolve(backend.outputs())?;
            Ok(Some(backend.capture_output(output)?))
        }
        Target::Region(output) => {
            let output = output.resolve(backend.outputs())?;
            Ok(backend.select_region(output)?)
        }
    }
}

fn capture_desktop(backend: &mut impl Backend) -> Result<CapturedImage, Error> {
    let outputs = sorted_outputs(backend);

    let screens = outputs
        .iter()
        .map(|output| {
            let area = output.logical.ok_or_else(|| Error::UnknownPlacement {
                name: output.name.clone(),
            })?;
            let image = backend.capture_output(output.id)?;

            Ok(Screen {
                name: &output.name,
                area,
                image,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    if sorted_outputs(backend) != outputs {
        return Err(Error::OutputsChanged);
    }

    compose(screens)
}

fn sorted_outputs(backend: &impl Backend) -> Vec<Output> {
    let mut outputs = backend.outputs().cloned().collect::<Vec<_>>();
    outputs.sort_unstable_by_key(|output| output.id.0);
    outputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some};
    use rstest::rstest;
    use std::num::NonZeroU32;
    use tvert_image::PixelSize;
    use tvert_wayland::{LogicalRect, PhysicalSize};

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Screen(OutputId),
        ScreenPicker,
        Region(OutputId),
    }

    #[derive(Debug, Clone, Copy)]
    enum Failure {
        Capture,
        Disconnect,
        Unplaced,
    }

    struct Fake {
        outputs: Vec<Output>,
        selection: Option<CapturedImage>,
        failure: Option<Failure>,
        calls: Vec<Call>,
    }

    impl Fake {
        /// Two 4x4 outputs side by side at one pixel per logical unit
        fn new(selection: Option<CapturedImage>) -> Self {
            let four = assert_some!(NonZeroU32::new(4));
            let output = |id, name: &str, x| Output {
                id: OutputId(id),
                name: name.to_owned(),
                size: PhysicalSize {
                    width: 4,
                    height: 4,
                },
                scale: 1,
                logical: Some(LogicalRect {
                    x,
                    y: 0,
                    width: four,
                    height: four,
                }),
            };

            Self {
                outputs: vec![output(1, "DP-1", 0), output(2, "HDMI-A-1", 4)],
                selection,
                failure: None,
                calls: Vec::new(),
            }
        }

        fn failing(failure: Failure) -> Self {
            let mut fake = Self::new(None);
            fake.failure = Some(failure);

            if let (Failure::Unplaced, Some(output)) = (failure, fake.outputs.get_mut(1)) {
                output.logical = None;
            }

            fake
        }
    }

    impl Backend for Fake {
        fn outputs(&self) -> impl Iterator<Item = &Output> {
            self.outputs.iter()
        }

        fn capture_output(
            &mut self,
            output: OutputId,
        ) -> Result<CapturedImage, tvert_wayland::Error> {
            self.calls.push(Call::Screen(output));

            match self.failure {
                Some(Failure::Capture) if output == OutputId(2) => {
                    return Err(tvert_wayland::Error::ScreencopyFailed);
                }
                Some(Failure::Disconnect) => {
                    self.outputs.retain(|output| output.id != OutputId(1));
                }
                _ => {}
            }

            Ok(image(4, 4))
        }

        fn select_screen(&mut self) -> Result<Option<CapturedImage>, tvert_wayland::Error> {
            self.calls.push(Call::ScreenPicker);
            Ok(self.selection.take())
        }

        fn select_region(
            &mut self,
            output: OutputId,
        ) -> Result<Option<CapturedImage>, tvert_wayland::Error> {
            self.calls.push(Call::Region(output));
            Ok(self.selection.take())
        }
    }

    fn image(width: u32, height: u32) -> CapturedImage {
        let size = PixelSize::new(
            assert_some!(NonZeroU32::new(width)),
            assert_some!(NonZeroU32::new(height)),
        );
        let len = assert_ok!(size.rgba_len());
        assert_ok!(CapturedImage::new(size, vec![0; len]))
    }

    fn named(name: &str) -> OutputSelection {
        OutputSelection::Named(name.to_owned())
    }

    #[rstest]
    #[case::named(
        Target::Screen(named("HDMI-A-1")),
        &[Call::Screen(OutputId(2))],
        4
    )]
    #[case::desktop(
        Target::Desktop,
        &[Call::Screen(OutputId(1)), Call::Screen(OutputId(2))],
        8
    )]
    fn screen_captures_the_selected_outputs(
        #[case] selection: Target,
        #[case] calls: &[Call],
        #[case] width: u32,
    ) {
        let mut backend = Fake::new(None);

        let captured = assert_ok!(capture_from(&mut backend, &selection));

        assert_eq!(assert_some!(captured).size(), image(width, 4).size());
        assert_eq!(backend.calls, calls);
    }

    #[rstest]
    #[case(Failure::Capture)]
    #[case(Failure::Disconnect)]
    #[case(Failure::Unplaced)]
    fn desktop_capture_never_returns_a_partial_image(#[case] failure: Failure) {
        let mut backend = Fake::failing(failure);

        let error = assert_err!(capture_from(&mut backend, &Target::Desktop));

        let expected = match failure {
            Failure::Capture => matches!(
                error,
                Error::Capture(tvert_wayland::Error::ScreencopyFailed)
            ),
            Failure::Disconnect => matches!(error, Error::OutputsChanged),
            Failure::Unplaced => {
                matches!(&error, Error::UnknownPlacement { name } if name == "HDMI-A-1")
            }
        };
        assert!(expected, "{failure:?}: {error:?}");
    }

    #[rstest]
    #[case::region_selected(
        Target::Region(named("DP-1")),
        Some(image(2, 1)),
        Call::Region(OutputId(1))
    )]
    #[case::region_cancelled(Target::Region(named("DP-1")), None, Call::Region(OutputId(1)))]
    #[case::screen_selected(
        Target::Screen(OutputSelection::All),
        Some(image(4, 4)),
        Call::ScreenPicker
    )]
    #[case::screen_cancelled(Target::Screen(OutputSelection::All), None, Call::ScreenPicker)]
    fn returns_the_selection_without_recapturing(
        #[case] target: Target,
        #[case] selection: Option<CapturedImage>,
        #[case] call: Call,
    ) {
        let expected = selection.as_ref().map(CapturedImage::size);
        let mut backend = Fake::new(selection);

        let captured = assert_ok!(capture_from(&mut backend, &target));

        assert_eq!(captured.as_ref().map(CapturedImage::size), expected);
        assert_eq!(backend.calls, [call]);
    }
}
