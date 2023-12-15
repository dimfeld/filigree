use std::path::PathBuf;

use clap::Parser;
use error_stack::Report;
use rayon::prelude::*;
use thiserror::Error;

use crate::{config::FullConfig, model::generator::ModelGenerator};

pub mod config;
mod format;
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
    #[error("Failed to run code formatter")]
    Formatter,
}

pub fn main() -> Result<(), Report<Error>> {
    let args = Cli::parse();

    let config_path = args.config.unwrap_or_else(|| PathBuf::from("filigree"));
    let config = FullConfig::from_dir(&config_path)?;

    let FullConfig { config, models } = config;

    let mut up_migrations = Vec::with_capacity(models.len());
    let mut down_migrations = Vec::with_capacity(models.len());

    let generators = models
        .into_iter()
        .map(|model| ModelGenerator::new(&config, model))
        .collect::<Vec<_>>();

    generators
        .par_iter()
        .try_for_each(|gen| gen.write_sql_queries())?;

    for generator in generators {
        up_migrations.push(generator.render_up_migration()?);
        down_migrations.push(generator.render_down_migration()?);
    }

    // TODO Write the migrations

    Ok(())
}
