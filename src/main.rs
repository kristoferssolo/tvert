use color_eyre::Result;
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};

fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init()?;

    for global in tvert_wayland::globals()? {
        println!("{global}");
    }

    for output in tvert_wayland::outputs()? {
        println!("{output}");
    }
    Ok(())
}
