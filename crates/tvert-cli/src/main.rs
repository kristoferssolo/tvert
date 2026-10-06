mod args;

use crate::args::{Args, Command};
use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use std::process::ExitCode;
use tracing::debug;
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};
use tvert_wayland::Wayland;
use tvert_workflow::capture_output_to_png;

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

    let output_name = match &args.command {
        Command::CaptureOutput { output, .. }
        | Command::ProbeOverlay { output }
        | Command::SelectRegion { output } => output,
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
        .find(|output| output.name == output_name.as_ref())
        .map(|output| output.id)
        .ok_or_else(|| eyre!("output not found: {output_name}"))?;

    debug!(?output_id, ?output_name, "selected output");

    match args.command {
        Command::CaptureOutput { path, .. } => {
            capture_output_to_png(&mut wayland, output_id, &path)?;
            println!("Saved {}", path.display());
        }
        Command::ProbeOverlay { .. } => {
            let configure = wayland.prepare_overlay(output_id)?;
            println!("Overlay configure: {configure} surface-local units");
        }
        Command::SelectRegion { .. } => {
            println!("Drag to select a region. Release to accept; right-click to cancel.");

            match wayland.select_region(output_id)? {
                Some(image) => println!("Selected {}", image.size()),
                None => println!("Selection cancelled"),
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
