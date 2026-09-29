use clap::Parser;

/// Native Wayland screenshot application
#[derive(Debug, Parser)]
#[command(name = "tvert", version, about)]
pub struct Args {
    /// Name of the output to select, such as DP-1
    #[arg(value_name = "OUTPUT")]
    pub output: String,
}
