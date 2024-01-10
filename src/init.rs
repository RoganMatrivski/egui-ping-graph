use clap::Parser;
use color_eyre::Report;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Verbosity log
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

const VERBOSE_LEVEL: &[&str] = &["info", "debug", "trace"];

macro_rules! get_this_pkg_name {
    () => {
        env!("CARGO_PKG_NAME").replace('-', "_")
    };
}

pub fn initialize() -> Result<Args, Report> {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    color_eyre::install()?;
    let args = Args::parse();

    let verbosity = match args.verbose {
        1..=3 => Some(VERBOSE_LEVEL[(args.verbose as usize) - 1]),
        _ => None,
    };

    let env_filter = EnvFilter::from_default_env()
        .add_directive(tracing::level_filters::LevelFilter::WARN.into());
    let env_filter = match verbosity {
        Some(v) => env_filter.add_directive(
            format!("{}={}", get_this_pkg_name!(), v)
                .parse()
                .expect("Failed to parse log parameter"),
        ),
        None => env_filter,
    };

    let fmt_layer = fmt::layer().with_writer(std::io::stderr);

    let fmt_layer = match verbosity {
        Some(_) => {
            // construct a layer that prints formatted traces to stderr
            fmt_layer
                .with_level(true) // include levels in formatted output
                .with_thread_ids(true) // include the thread ID of the current thread
                .with_thread_names(true) // include the name of the current thread
        }
        None => {
            // construct a layer that prints formatted traces to stderr
            fmt_layer
        }
    };

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(env_filter)
        .with(ErrorLayer::default())
        .init();

    Ok(args)
}
