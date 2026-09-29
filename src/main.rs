mod args;

use crate::args::Args;
use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use std::process::ExitCode;
use tracing::debug;
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};
use tvert_wayland::Wayland;

fn main() -> Result<ExitCode> {
    color_eyre::install()?;

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            error.print()?;

            if error.use_stderr() {
                return Ok(ExitCode::from(2));
            }
            return Ok(ExitCode::SUCCESS);
        }
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init()?;

    let mut wayland = Wayland::connect()?;

    for global in wayland.globals() {
        println!("{global}");
    }

    println!(" ");

    for output in wayland.outputs() {
        println!("{output}");
    }

    let output_id = wayland
        .outputs()
        .find(|output| output.name == args.output)
        .map(|output| output.id)
        .ok_or_else(|| eyre!("output not found: {}", args.output))?;

    debug!(
        ?output_id,
        output_name = %args.output,
        "selected output"
    );

    let image = wayland.capture_output(output_id)?;

    debug!(
        width = image.width,
        height = image.height,
        bytes = image.rgba.len(),
        "read captured output into RGBA image"
    );

    Ok(ExitCode::SUCCESS)
}
