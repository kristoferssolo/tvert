use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tvert_workflow::{Destination, OutputSelection, Target};

use crate::config::{CaptureSettings, SaveSettings};

const EXAMPLES: &str = r"
Examples:
  tvert preview region
  tvert preview screen --output DP-1
  tvert save region --path shot.png
  tvert config check --config ~/.config/tvert/config.toml
";

/// Native Wayland screenshot application
#[derive(Debug, Parser)]
#[command(name = "tvert", version, about, after_help=EXAMPLES)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    /// Read configuration from this TOML file
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Capture and open the editor to copy or save
    #[command(subcommand)]
    Preview(TargetCommand),

    /// Capture and save immediately
    #[command(subcommand)]
    Save(TargetCommand),

    /// Inspect configuration without connecting to Wayland
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Development diagnostics
    #[command(subcommand)]
    Debug(DebugCommand),
}

#[derive(Debug, Subcommand)]
pub enum TargetCommand {
    /// Capture an entire screen
    Screen(TargetArgs),
    /// Drag to select part of the screen. Right-click to cancel
    Region(TargetArgs),
}

#[derive(Debug, clap::Args)]
pub struct TargetArgs {
    /// Output name, such as DP-1, or "all"; overrides capture.output
    #[arg(long, value_name = "NAME")]
    output: Option<OutputSelection>,

    /// PNG file to create; overrides save.path
    ///
    /// An explicit file must not already exist, and its directory is not
    /// created. Otherwise, Tvert expands save.path, creates its directories
    /// if needed, and appends a numeric suffix if the generated name is taken.
    #[arg(long, value_name = "FILE")]
    path: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Validate configuration
    Check,
    /// Print the default configuration as TOML
    PrintDefault,
}

#[derive(Debug, Subcommand)]
pub enum DebugCommand {
    /// Configure a selection overlay without displaying it
    Overlay {
        /// Output name, such as DP-1, or "all"; overrides capture.output
        #[arg(long, value_name = "NAME")]
        output: Option<OutputSelection>,
    },
}

impl TargetCommand {
    pub fn resolve(self, capture: &CaptureSettings, save: &SaveSettings) -> (Target, Destination) {
        let (target, args): (fn(OutputSelection) -> Target, _) = match self {
            Self::Screen(args) => (Target::Screen, args),
            Self::Region(args) => (Target::Region, args),
        };

        let output = args.output.unwrap_or_else(|| capture.output.clone());
        (
            target(output),
            Destination::new(args.path, save.path.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use rstest::rstest;
    use tvert_workflow::PathPattern;

    #[rstest]
    #[case(
        &["save", "region"],
        Target::Region(OutputSelection::Named("DP-1".to_owned())),
        None
    )]
    #[case(
        &["preview", "screen", "--output", "all", "--path", "shot.png"],
        Target::Screen(OutputSelection::All),
        Some("shot.png")
    )]
    fn flags_override_configuration(
        #[case] args: &[&str],
        #[case] target: Target,
        #[case] path: Option<&str>,
    ) {
        let capture = CaptureSettings {
            output: OutputSelection::Named("DP-1".to_owned()),
        };
        let save = SaveSettings {
            path: assert_ok!(PathPattern::new("shots/%Y.png", None)),
        };
        let args = assert_ok!(Args::try_parse_from([&["tvert"], args].concat()));
        let (Command::Preview(command) | Command::Save(command)) = args.command else {
            panic!("expected a capture command");
        };

        assert_eq!(
            command.resolve(&capture, &save),
            (target, Destination::new(path.map(PathBuf::from), save.path))
        );
    }
}
