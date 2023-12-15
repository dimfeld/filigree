use std::path::PathBuf;

use clap::Parser;
use config::Config;
use error_stack::{Report, ResultExt};
use thiserror::Error;

use crate::config::FullConfig;

pub mod config;
pub mod model;
pub mod templates;

#[derive(Parser)]
pub struct Cli {
    /// Override the path to the configuration directory. By default this looks for ./filigree
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to parse configuration file")]
    Config,
    #[error("Failed to read configuration file")]
    ReadConfigFile,
    #[error("Failed to write file")]
    WriteFile,
    #[error("Failed to render template")]
    Render,
}

pub fn main() -> Result<(), Report<Error>> {
    let args = Cli::parse();

    let config_path = args.config.unwrap_or_else(|| PathBuf::from("filigree"));
    let config = FullConfig::from_dir(&config_path)?;

    println!("config: {:?}", config);

    Ok(())
}
