use color_eyre::{Result, eyre::eyre};
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};
use tvert_wayland::Wayland;

fn main() -> Result<()> {
    color_eyre::install()?;

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

    let output_name = std::env::args()
        .nth(1)
        .ok_or_else(|| eyre!("usage: tvert <output-name>"))?;

    let output_id = wayland
        .outputs()
        .find(|output| output.name == output_name)
        .map(|output| output.id)
        .ok_or_else(|| eyre!("output not found: {output_name}"))?;

    wayland.create_output_source(output_id)?;

    Ok(())
}
