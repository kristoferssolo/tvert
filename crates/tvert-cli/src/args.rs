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
    CaptureOutput {
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
    /// Display a frozen output preview. Right-click to close
    PreviewOutput {
        #[arg(value_name = "OUTPUT")]
        output: String,
    },
}
