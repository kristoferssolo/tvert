use crate::{Error, target::Target};
use tvert_image::CapturedImage;
use tvert_wayland::{Output, OutputId, Wayland};

/// The compositor operations capture needs, separated for testing
trait Backend {
    fn outputs(&self) -> impl Iterator<Item = &Output>;
    fn capture_output(&mut self, output: OutputId) -> Result<CapturedImage, tvert_wayland::Error>;
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

    fn select_region(
        &mut self,
        output: OutputId,
    ) -> Result<Option<CapturedImage>, tvert_wayland::Error> {
        self.select_region(output)
    }
}

/// Connects to the compositor and captures `target` into an image.
///
/// Region capture waits for the user's selection. Returns `None` if the user
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::OutputSelection;
    use claims::{assert_ok, assert_some};
    use rstest::rstest;
    use std::num::NonZeroU32;
    use tvert_image::PixelSize;
    use tvert_wayland::PhysicalSize;

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Screen(OutputId),
        Region(OutputId),
    }

    struct Fake {
        outputs: Vec<Output>,
        selection: Option<CapturedImage>,
        calls: Vec<Call>,
    }

    impl Fake {
        fn new(selection: Option<CapturedImage>) -> Self {
            let output = |id, name: &str| Output {
                id: OutputId(id),
                name: name.to_owned(),
                size: PhysicalSize {
                    width: 4,
                    height: 4,
                },
                scale: 1,
            };

            Self {
                outputs: vec![output(1, "DP-1"), output(2, "HDMI-A-1")],
                selection,
                calls: Vec::new(),
            }
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
            Ok(image(4, 4))
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

    #[test]
    fn screen_captures_the_named_output() {
        let mut backend = Fake::new(None);

        let captured = assert_ok!(capture_from(
            &mut backend,
            &Target::Screen(named("HDMI-A-1"))
        ));

        assert_some!(captured);
        assert_eq!(backend.calls, [Call::Screen(OutputId(2))]);
    }

    #[rstest]
    #[case::selected(Some(image(2, 1)))]
    #[case::cancelled(None)]
    fn region_returns_the_selection_without_recapturing(#[case] selection: Option<CapturedImage>) {
        let expected = selection.as_ref().map(CapturedImage::size);
        let mut backend = Fake::new(selection);

        let captured = assert_ok!(capture_from(&mut backend, &Target::Region(named("DP-1"))));

        assert_eq!(captured.as_ref().map(CapturedImage::size), expected);
        assert_eq!(backend.calls, [Call::Region(OutputId(1))]);
    }
}
