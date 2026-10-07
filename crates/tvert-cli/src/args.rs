use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Native Wayland screenshot application
#[derive(Debug, Parser)]
#[command(name = "tvert", version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Capture a monitor and save it as a PNG. Right-click to cancel
    CaptureOutput {
        /// Output name, such as DP-1
        #[arg(value_name = "OUTPUT")]
        output: String,

        /// Destination PNG file. Must not already exist
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    /// Capture a region and save it as a PNG. Right-click to cancel
    CaptureRegion {
        /// Output name, such as DP-1
        #[arg(value_name = "OUTPUT")]
        output: String,

        /// Destination PNG file. Must not already exist
        #[arg(value_name = "FILE")]
        path: PathBuf,
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
}
