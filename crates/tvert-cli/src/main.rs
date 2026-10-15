mod args;
mod config;
mod error;

use crate::{
    args::{Args, Command, ConfigCommand, DebugCommand, TargetCommand},
    config::{AppConfig, CaptureSettings},
    error::ConfigError,
};
use clap::Parser;
use miette::{Diagnostic, IntoDiagnostic, MietteHandlerOpts, ReportHandler};
use std::{cell::RefCell, error::Error as StdError, fmt::Debug, process::ExitCode, rc::Rc};
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};
use tvert_editor::{ExportAction, Settings};
use tvert_image::CapturedImage;
use tvert_wayland::Wayland;
use tvert_workflow::{Destination, Target, prepare_png_copy, save_png};

#[derive(Debug, Clone, Copy)]
enum Outcome {
    Preview,
    Save,
}

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

    let config_path = args.config.as_deref();
    match args.command {
        Command::Config(ConfigCommand::Check) => {
            config::load(config_path)?;
            println!("Configuration is valid");
        }
        Command::Config(ConfigCommand::PrintDefault) => print!("{}", config::DEFAULT_CONFIG),
        Command::Debug(command) => run_debug(command, config::load(config_path)?.capture)?,
        Command::Preview(command) => {
            run_capture(Outcome::Preview, command, &config::load(config_path)?)?;
        }
        Command::Save(command) => run_capture(Outcome::Save, command, &config::load(config_path)?)?,
    }

    Ok(ExitCode::SUCCESS)
}

fn init_tracing() -> miette::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init()
        .into_diagnostic()
}

fn run_capture(outcome: Outcome, command: TargetCommand, config: &AppConfig) -> miette::Result<()> {
    init_tracing()?;

    let (target, destination) = command.resolve(&config.capture, &config.save);
    match &target {
        Target::Screen(_) => {
            println!("Click a monitor to capture it; right-click to canel.");
        }
        Target::Region(_) => {
            println!("Drag to select a region. Release to accept; right-click to cancel.");
        }
        Target::Desktop => {}
    }

    let Some(image) = tvert_workflow::capture(&target).into_diagnostic()? else {
        println!("Selection cancelled");
        return Ok(());
    };

    match outcome {
        Outcome::Preview => preview(image, config.editor, destination),
        Outcome::Save => {
            let saved = save_png(&destination, &image).into_diagnostic()?;
            println!("Saved {} to {}", image.size(), saved.display());
            Ok(())
        }
    }
}

/// Opens the editor, then serves any copied image until the clipboard is replaced
fn preview(
    image: CapturedImage,
    settings: Settings,
    destination: Destination,
) -> miette::Result<()> {
    let clipboard = Rc::new(RefCell::new(None));
    let editor_clipboard = clipboard.clone();

    tvert_editor::run(
        image,
        settings,
        move |action, image| -> Result<(), tvert_workflow::Error> {
            let offer = match action {
                ExportAction::Copy | ExportAction::CopyAndSave => Some(prepare_png_copy(image)?),
                ExportAction::Save => None,
            };

            if matches!(action, ExportAction::Save | ExportAction::CopyAndSave) {
                let saved = save_png(&destination, image)?;
                println!("Saved {} to {}", image.size(), saved.display());
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

    Ok(())
}

fn run_debug(command: DebugCommand, capture: CaptureSettings) -> miette::Result<()> {
    init_tracing()?;

    let DebugCommand::Overlay { output } = command;
    let mut wayland = Wayland::connect().into_diagnostic()?;
    let output_id = output
        .unwrap_or(capture.output)
        .resolve(wayland.outputs())
        .into_diagnostic()?;

    let configure = wayland.prepare_overlay(output_id).into_diagnostic()?;
    println!("Overlay configure: {configure} surface-local units");

    Ok(())
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
