use crate::{
    Error, OutputSelection,
    desktop::{Screen, compose},
    target::Target,
};
use tvert_image::CapturedImage;
use tvert_wayland::{FrozenOutput, Output, OutputId, RegionSelection, Wayland};

/// The compositor operations capture needs, separated for testing
trait Backend {
    fn outputs(&self) -> impl Iterator<Item = &Output>;
    fn capture_output(&mut self, output: OutputId) -> Result<CapturedImage, tvert_wayland::Error>;
    fn freeze_outputs(&mut self) -> Result<Vec<FrozenOutput>, tvert_wayland::Error>;
    fn select_screen(&mut self) -> Result<Option<CapturedImage>, tvert_wayland::Error>;
    fn select_region(
        &mut self,
        output: OutputId,
    ) -> Result<Option<CapturedImage>, tvert_wayland::Error>;
    fn select_desktop_region(&mut self) -> Result<Option<RegionSelection>, tvert_wayland::Error>;
}

impl Backend for Wayland {
    fn outputs(&self) -> impl Iterator<Item = &Output> {
        self.outputs()
    }

    fn capture_output(&mut self, output: OutputId) -> Result<CapturedImage, tvert_wayland::Error> {
        self.capture_output(output)
    }

    fn freeze_outputs(&mut self) -> Result<Vec<FrozenOutput>, tvert_wayland::Error> {
        self.freeze_outputs()
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

    fn select_desktop_region(&mut self) -> Result<Option<RegionSelection>, tvert_wayland::Error> {
        self.select_desktop_region()
    }
}

/// Connects to the compositor and captures `target` into an image.
///
/// Screen and region capture can wait for selection. Returns `None` if the user
/// cancels. Region capture with [`OutputSelection::All`] on several outputs
/// crops the desktop composed from the captures frozen for selection. Never
/// touches the filesystem, and closes the connection before returning so the
/// image can be handed to the editor.
///
/// # Errors
///
/// Returns any error from [`crate::OutputSelection::resolve`], desktop
/// composition, or cropping, or [`Error::Capture`] if connecting, capturing,
/// or selecting fails.
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
        Target::Region(OutputSelection::All) if backend.outputs().count() > 1 => {
            capture_desktop_region(backend)
        }
        Target::Region(output) => {
            let output = output.resolve(backend.outputs())?;
            Ok(backend.select_region(output)?)
        }
    }
}

fn capture_desktop(backend: &mut impl Backend) -> Result<CapturedImage, Error> {
    let frozen = backend.freeze_outputs()?;
    Ok(compose(screens(frozen)?)?.into_image())
}

fn capture_desktop_region(backend: &mut impl Backend) -> Result<Option<CapturedImage>, Error> {
    let Some(selection) = backend.select_desktop_region()? else {
        return Ok(None);
    };
    compose(screens(selection.outputs)?)?
        .crop(selection.bounds)
        .map(Some)
}

fn screens(frozen: Vec<FrozenOutput>) -> Result<Vec<Screen>, Error> {
    frozen
        .into_iter()
        .map(|frozen| {
            let (output, image) = frozen.into_parts()?;
            let area = output.logical.ok_or_else(|| Error::UnknownPlacement {
                name: output.name.clone(),
            })?;
            Ok(Screen {
                name: output.name,
                area,
                image,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some};
    use rstest::rstest;
    use std::num::NonZeroU32;
    use tvert_image::{ImageTransform, PixelSize};
    use tvert_wayland::{LogicalRect, PhysicalSize, RegionBounds};

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Screen(OutputId),
        Freeze,
        ScreenPicker,
        Region(OutputId),
        DesktopRegion,
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
        region: Option<RegionSelection>,
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
                region: None,
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

        fn fail(&self) -> Result<(), tvert_wayland::Error> {
            match self.failure {
                Some(Failure::Capture) => Err(tvert_wayland::Error::ScreencopyFailed),
                Some(Failure::Disconnect) => Err(tvert_wayland::Error::OutputsChanged),
                _ => Ok(()),
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

        fn freeze_outputs(&mut self) -> Result<Vec<FrozenOutput>, tvert_wayland::Error> {
            self.calls.push(Call::Freeze);
            self.fail()?;
            Ok(self
                .outputs
                .iter()
                .cloned()
                .map(|output| FrozenOutput {
                    output,
                    image: image(4, 4),
                    orientation: ImageTransform::Normal,
                })
                .collect())
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

        fn select_desktop_region(
            &mut self,
        ) -> Result<Option<RegionSelection>, tvert_wayland::Error> {
            self.calls.push(Call::DesktopRegion);
            self.fail()?;
            if let Some(output) = self.outputs.iter().find(|output| output.logical.is_none()) {
                return Err(tvert_wayland::Error::UnknownPlacement {
                    name: output.name.clone(),
                });
            }
            Ok(self.region.take())
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

    fn frozen(
        name: &str,
        [x, y, width, height]: [i32; 4],
        image: CapturedImage,
        orientation: ImageTransform,
    ) -> FrozenOutput {
        let len = |value| assert_some!(u32::try_from(value).ok().and_then(NonZeroU32::new));
        let (buffer_width, buffer_height) = image.size().into();
        let output = Output {
            id: OutputId(0),
            name: name.to_owned(),
            size: PhysicalSize {
                width: buffer_width,
                height: buffer_height,
            },
            scale: 1,
            logical: Some(LogicalRect {
                x,
                y,
                width: len(width),
                height: len(height),
            }),
        };

        FrozenOutput {
            output,
            image,
            orientation,
        }
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
    #[case::desktop(Target::Desktop, &[Call::Freeze], 8)]
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
    fn desktop_capture_never_returns_a_partial_image(
        #[values(Target::Desktop, Target::Region(OutputSelection::All))] target: Target,
        #[values(Failure::Capture, Failure::Disconnect, Failure::Unplaced)] failure: Failure,
    ) {
        let mut backend = Fake::failing(failure);

        let error = assert_err!(capture_from(&mut backend, &target));

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
    #[case::desktop_region_cancelled(
        Target::Region(OutputSelection::All),
        None,
        Call::DesktopRegion
    )]
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

    #[test]
    fn a_single_output_region_needs_no_desktop_placement() {
        let mut backend = Fake::new(Some(image(2, 1)));
        backend.outputs.truncate(1);
        assert_some!(backend.outputs.first_mut()).logical = None;

        let captured = assert_ok!(capture_from(
            &mut backend,
            &Target::Region(OutputSelection::All)
        ));

        assert_eq!(assert_some!(captured).size(), image(2, 1).size());
        assert_eq!(backend.calls, [Call::Region(OutputId(1))]);
    }

    // Desktop, one logical unit per pixel; the crop is x = 1..4, y = 0..2:
    //
    // -2 -1  0  1
    //  A  B  .  .   y = 0, "DP-1" is captured as a 1x2 column, rotated
    //  .  .  .  C   y = 1, "HDMI-A-1"
    #[test]
    fn all_output_regions_crop_the_selected_captures_without_recapturing() {
        const A: [u8; 4] = [10, 20, 30, 255];
        const B: [u8; 4] = [40, 50, 60, 128];
        const C: [u8; 4] = [70, 80, 90, 64];
        const CLEAR: [u8; 4] = [0; 4];

        let rgba = |width, height, pixels: &[[u8; 4]]| {
            let size = assert_ok!(PixelSize::try_from((width, height)));
            assert_ok!(CapturedImage::new(size, pixels.concat()))
        };
        let outputs = vec![
            frozen(
                "DP-1",
                [-2, 0, 2, 1],
                rgba(1, 2, &[B, A]),
                ImageTransform::Rotate270,
            ),
            frozen(
                "HDMI-A-1",
                [1, 1, 1, 1],
                rgba(1, 1, &[C]),
                ImageTransform::Normal,
            ),
        ];
        let mut backend = Fake::new(None);
        backend.region = Some(RegionSelection {
            outputs,
            bounds: assert_some!(RegionBounds::new(-0.5, 0.5, 1.5, 1.5)),
        });

        let captured = assert_ok!(capture_from(
            &mut backend,
            &Target::Region(OutputSelection::All)
        ));

        let captured = assert_some!(captured);
        assert_eq!(captured.size(), image(3, 2).size());
        assert_eq!(captured.rgba(), [B, CLEAR, CLEAR, CLEAR, CLEAR, C].concat());
        assert_eq!(backend.calls, [Call::DesktopRegion]);
    }
}
