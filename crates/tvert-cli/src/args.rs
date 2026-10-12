use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Native Wayland screenshot application
#[derive(Debug, Parser)]
#[command(name = "tvert", version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    /// Read configuration from this TOML file
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Validate configuration without connecting to Wayland
    CheckConfig,

    /// Print the embeded default configuration as TOML
    PrintDefaultConfig,

    #[command(flatten)]
    Capture(CaptureCommand),
}

#[derive(Debug, Subcommand)]
pub enum CaptureCommand {
    /// Capture a monitor and save it as a PNG. Right-click to cancel
    CaptureOutput {
        /// Output name, such as DP-1
        #[arg(value_name = "OUTPUT")]
        output: String,

        /// PNG file to create instead of expanding save.path
        ///
        /// An explicit file must not already exist, and its directory is not
        /// created. When omitted, Tvert expands save.path, creates its
        /// directories if needed, and appends a numeric suffix if the
        /// generated name is taken.
        #[arg(value_name = "FILE")]
        path: Option<PathBuf>,
    },
    /// Capture a region and save it as a PNG. Right-click to cancel
    CaptureRegion {
        /// Output name, such as DP-1
        #[arg(value_name = "OUTPUT")]
        output: String,

        /// PNG file to create instead of expanding save.path
        ///
        /// An explicit file must not already exist, and its directory is not
        /// created. When omitted, Tvert expands save.path, creates its
        /// directories if needed, and appends a numeric suffix if the
        /// generated name is taken.
        #[arg(value_name = "FILE")]
        path: Option<PathBuf>,
    },
    /// Check overlay configuration without displaying it.
    ProbeOverlay {
        /// Output name, such as DP-1.
        #[arg(value_name = "OUTPUT")]
        output: String,
    },
    /// Capture a region of an output. Right-click to cancel.
    SelectRegion {
        #[arg(value_name = "OUTPUT")]
        output: String,
    },
    /// Select a region and open it in the editor. Right-click to cancel
    EditRegion {
        /// Output name, such as DP-1
        #[arg(value_name = "OUTPUT")]
        output: String,

        /// PNG file to create instead of expanding save.path
        ///
        /// An explicit file must not already exist, and its directory is not
        /// created. When omitted, Tvert expands save.path, creates its
        /// directories if needed, and appends a numeric suffix if the
        /// generated name is taken.
        #[arg(value_name = "FILE")]
        path: Option<PathBuf>,
    },
}
