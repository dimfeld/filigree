use std::path::PathBuf;

use clap::Parser;
use config::Config;
use error_stack::{Report, ResultExt};
use thiserror::Error;

pub mod config;
pub mod model;
pub mod templates;

#[derive(Parser)]
pub struct Cli {
    /// Override the path to the configuration file. By default this looks for filigree.toml
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to parse config")]
    Config,
}

pub fn main() -> Result<(), Report<Error>> {
    let args = Cli::parse();

    let config_path = args
        .config
        .unwrap_or_else(|| PathBuf::from("filigree.toml"));
    let config_data = std::fs::read_to_string(&config_path)
        .change_context(Error::Config)
        .attach_printable_lazy(|| config_path.display().to_string())?;
    let config: Config = toml::from_str(&config_data).change_context(Error::Config)?;

    println!("config: {:?}", config);

    Ok(())
}
