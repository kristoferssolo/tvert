mod args;
mod config;
mod error;

use crate::{
    args::{Args, CaptureCommand, Command},
    error::ConfigError,
};
use clap::Parser;
use miette::{Diagnostic, IntoDiagnostic, MietteHandlerOpts, ReportHandler, miette};
use std::{cell::RefCell, error::Error as StdError, process::ExitCode, rc::Rc};
use tracing::debug;
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};
use tvert_editor::{ExportAction, Settings};
use tvert_wayland::Wayland;
use tvert_workflow::{capture_output_to_png, prepare_png_copy, save_png};

fn main() -> miette::Result<ExitCode> {
    miette::set_hook(Box::new(reported_handler))?;

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            error.print().into_diagnostic()?;

            if error.use_stderr() {
                return Ok(ExitCode::from(2));
            }
            return Ok(ExitCode::SUCCESS);
        }
    };

    match args.command {
        Command::CheckConfig => {
            println!("Configuration is valid");
            Ok(ExitCode::SUCCESS)
        }
        Command::PrintDefaultConfig => {
            print!("{}", config::DEFAULT_CONFIG);
            Ok(ExitCode::SUCCESS)
        }
        Command::Capture(command) => {
            let settings = config::load(args.config.as_deref())?;
            run_capture(command, settings)
        }
    }
}

fn run_capture(command: CaptureCommand, settings: Settings) -> miette::Result<ExitCode> {
    let output_name = match &command {
        CaptureCommand::CaptureOutput { output, .. }
        | CaptureCommand::CaptureRegion { output, .. }
        | CaptureCommand::EditRegion { output, .. }
        | CaptureCommand::ProbeOverlay { output }
        | CaptureCommand::SelectRegion { output } => output,
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init()
        .into_diagnostic()?;

    let mut wayland = Wayland::connect().into_diagnostic()?;

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
        .ok_or_else(|| miette!("output not found: {output_name}"))?;

    debug!(?output_id, ?output_name, "selected output");

    match command {
        CaptureCommand::CaptureOutput { path, .. } => {
            capture_output_to_png(&mut wayland, output_id, &path).into_diagnostic()?;
            println!("Saved {}", path.display());
        }
        CaptureCommand::CaptureRegion { path, .. } => {
            println!("Drag to select a region. Release to accept; right-click to cancel.");

            let Some(image) = wayland.select_region(output_id).into_diagnostic()? else {
                println!("Selection cancelled");
                return Ok(ExitCode::SUCCESS);
            };

            save_png(&path, &image).into_diagnostic()?;
            println!("Saved {} to {}", image.size(), path.display());
        }
        CaptureCommand::EditRegion { path, .. } => {
            println!("Drag to select a region. Release to accept; right-click to cancel.");

            let Some(image) = wayland.select_region(output_id).into_diagnostic()? else {
                println!("Selection cancelled");
                return Ok(ExitCode::SUCCESS);
            };

            drop(wayland);

            let clipboard = Rc::new(RefCell::new(None));
            let editor_clipboard = clipboard.clone();

            tvert_editor::run(
                image,
                settings,
                move |action, image| -> Result<(), tvert_workflow::Error> {
                    let offer = match action {
                        ExportAction::Copy | ExportAction::CopyAndSave => {
                            Some(prepare_png_copy(image)?)
                        }
                        ExportAction::Save => None,
                    };

                    if matches!(action, ExportAction::Save | ExportAction::CopyAndSave) {
                        save_png(&path, image)?;
                        println!("Saved {} to {}", image.size(), path.display());
                    }

                    if let Some(offer) = offer {
                        editor_clipboard.replace(Some(offer));
                    }

                    Ok(())
                },
            )
            .into_diagnostic()?;

            if let Some(offer) = clipboard.take() {
                println!("Serving the copied image until the clipboard is replaced.");
                offer.serve().into_diagnostic()?;
            }
        }
        CaptureCommand::ProbeOverlay { .. } => {
            let configure = wayland.prepare_overlay(output_id).into_diagnostic()?;
            println!("Overlay configure: {configure} surface-local units");
        }
        CaptureCommand::SelectRegion { .. } => {
            println!("Drag to select a region. Release to accept; right-click to cancel.");

            match wayland.select_region(output_id).into_diagnostic()? {
                Some(image) => println!("Selected {}", image.size()),
                None => println!("Selection cancelled"),
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn reported_handler(diagnostic: &(dyn Diagnostic + 'static)) -> Box<dyn ReportHandler> {
    let error: &(dyn StdError + 'static) = diagnostic;
    let options = MietteHandlerOpts::new();

    let options = if matches!(
        error.downcast_ref::<ConfigError>(),
        Some(ConfigError::Deserialize(_))
    ) {
        options.without_cause_chain()
    } else {
        options
    };

    Box::new(options.build())
}
