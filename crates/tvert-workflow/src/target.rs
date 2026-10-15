use crate::{EmptyOutputName, Error};
use std::str::FromStr;
use tvert_wayland::{Output, OutputId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Every output, combined without interactive selection
    Desktop,
    /// An entire output
    Screen(OutputSelection),
    /// A region the user drags out on an output
    Region(OutputSelection),
}

/// The output a capture uses, parsed from `all` or an output name
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputSelection {
    /// Every output, combined according to the desktop layout
    ///
    /// Region selection cannot span outputs yet, so it accepts this only when
    /// exactly one output exists.
    All,
    /// The output with this name, such as `DP-1`
    Named(String),
}

impl OutputSelection {
    /// Resolves a selection that requires exactly one output.
    ///
    /// Combined capture handles `All` separately; this method never chooses an
    /// arbitrary output when several are available.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoOutputs`] if `outputs` is empty,
    /// [`Error::UnknownOutput`] if no output has the requested name, or
    /// [`Error::CrossScreenRegion`] if `All` spans several outputs.
    pub fn resolve<'a>(
        &self,
        outputs: impl IntoIterator<Item = &'a Output>,
    ) -> Result<OutputId, Error> {
        let outputs = outputs.into_iter().collect::<Vec<_>>();
        let names = || {
            let mut names = outputs
                .iter()
                .map(|output| output.name.clone())
                .collect::<Vec<_>>();
            names.sort_unstable();
            names
        };

        match (self, outputs.as_slice()) {
            (_, []) => Err(Error::NoOutputs),
            (Self::All, [output]) => Ok(output.id),
            (Self::All, _) => Err(Error::CrossScreenRegion { available: names() }),
            (Self::Named(name), _) => outputs
                .iter()
                .find(|output| &output.name == name)
                .map(|output| output.id)
                .ok_or_else(|| Error::UnknownOutput {
                    name: name.clone(),
                    available: names(),
                }),
        }
    }
}

impl FromStr for OutputSelection {
    type Err = EmptyOutputName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Err(EmptyOutputName),
            "all" => Ok(Self::All),
            name => Ok(Self::Named(name.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_matches, assert_ok_eq};
    use rstest::rstest;
    use tvert_wayland::PhysicalSize;

    fn output(id: u32, name: &str) -> Output {
        Output {
            id: OutputId(id),
            name: name.to_owned(),
            size: PhysicalSize {
                width: 1920,
                height: 1080,
            },
            scale: 1,
            logical: None,
        }
    }

    #[rstest]
    #[case(OutputSelection::All, vec![output(1, "DP-1")], OutputId(1))]
    #[case(
        OutputSelection::Named("HDMI-A-1".to_owned()),
        vec![output(1, "DP-1"), output(2, "HDMI-A-1")],
        OutputId(2)
    )]
    fn selection_resolves_to_one_output(
        #[case] selection: OutputSelection,
        #[case] outputs: Vec<Output>,
        #[case] expected: OutputId,
    ) {
        assert_ok_eq!(selection.resolve(&outputs), expected);
    }

    #[test]
    fn unresolvable_selections_explain_the_available_outputs() {
        let outputs = [output(2, "HDMI-A-1"), output(1, "DP-1")];
        let names = vec!["DP-1".to_owned(), "HDMI-A-1".to_owned()];

        assert_matches!(
            OutputSelection::All.resolve(&outputs),
            Err(Error::CrossScreenRegion { available }) if available == names
        );
        assert_matches!(
            OutputSelection::Named("DP-2".to_owned()).resolve(&outputs),
            Err(Error::UnknownOutput { name, available }) if name == "DP-2" && available == names
        );
        assert_matches!(OutputSelection::All.resolve(&[]), Err(Error::NoOutputs));
    }
}
