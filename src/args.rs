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
        /// Name of the output to select, such as DP-1
        #[arg(value_name = "OUTPUT")]
        output: String,

        /// Destination PNG file. Must not already exist
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
}
