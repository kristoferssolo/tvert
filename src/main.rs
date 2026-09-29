mod args;
mod export;

use crate::{
    args::{Args, Command},
    export::save_png,
};
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

    let Command::CaptureOutput {
        output: output_name,
        path,
    } = args.command;

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
        .find(|output| output.name == output_name)
        .map(|output| output.id)
        .ok_or_else(|| eyre!("output not found: {output_name}"))?;

    debug!(
        ?output_id,
        output_name = %output_name,
        ?output_name,
        "selected output"
    );

    let image = wayland.capture_output(output_id)?;

    debug!(
        width = image.width,
        height = image.height,
        bytes = image.rgba.len(),
        "read captured output into RGBA image"
    );

    save_png(&path, &image)?;
    println!("Saved {}", path.display());

    Ok(ExitCode::SUCCESS)
}
