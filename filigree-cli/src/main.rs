use std::path::PathBuf;

use clap::Parser;
use error_stack::Report;
use model::Model;
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

fn build_models(mut config_models: Vec<Model>) -> Vec<Model> {
    let mut models = Model::create_default_models();
    // See if any of the built-in models have been customized
    for model in models.iter_mut() {
        let same_model = config_models.iter().position(|m| m.name == model.name);
        let Some(position) = same_model else {
            continue;
        };

        let same_model = config_models.remove(position);
        model.merge_from(same_model);
    }

    models.extend(config_models.into_iter());
    models
}

pub fn main() -> Result<(), Report<Error>> {
    let args = Cli::parse();

    let config_path = args.config.unwrap_or_else(|| PathBuf::from("filigree"));
    let config = FullConfig::from_dir(&config_path)?;

    let FullConfig {
        config,
        models: config_models,
    } = config;

    let models = build_models(config_models);

    let generators = models
        .into_iter()
        .map(|model| ModelGenerator::new(&config, model))
        .collect::<Vec<_>>();

    generators
        .par_iter()
        .try_for_each(|gen| gen.write_sql_queries())?;

    let up_migrations = generators
        .iter()
        .map(|gen| gen.render_up_migration())
        .collect::<Result<Vec<_>, _>>()?;

    let down_migrations = generators
        .iter()
        .map(|gen| gen.render_down_migration())
        .collect::<Result<Vec<_>, _>>()?;

    // TODO Write the migrations

    Ok(())
}
