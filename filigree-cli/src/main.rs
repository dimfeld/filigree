use std::{error::Error as _, path::PathBuf};

use clap::{Parser, Subcommand};
use error_stack::Report;
use thiserror::Error;

use crate::config::FullConfig;

mod add_deps;
mod config;
mod format;
mod init;
mod merge_files;
mod migrations;
mod model;
mod print_env;
mod root;
mod state;
mod templates;
mod write;

#[derive(Parser)]
#[clap(about, version)]
pub struct Cli {
    /// Override the path to the configuration directory. By default this looks for ./filigree
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print all the environment variables used by the built application
    Env(print_env::Command),
    /// Generate application code from the configuration files
    Write(write::Command),
    /// Create a new project using Filigree
    Init(init::Command),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to parse configuration file")]
    Config,
    #[error("Failed to read configuration file")]
    ReadConfigFile,
    #[error("Failed to read migration files")]
    ReadMigrationFiles,
    #[error("Failed to write file")]
    WriteFile,
    #[error("{0}{}", .0.source().map(|e| format!("\n{e:?}")).unwrap_or_default())]
    Render(#[from] tera::Error),
    #[error("Failed to run code formatter")]
    Formatter,
    #[error("Failed to run cargo")]
    Cargo,
    #[error("Failed to run npm")]
    Npm,
    #[error("Failed to run psql")]
    Psql,
    #[error("Input error")]
    Input,
    #[error("Missing model {0} in model {1} field {2}")]
    MissingModel(String, String, String),
    #[error("Model {0} has {1} but {1} has no belongs_to setting")]
    MissingBelongsTo(String, String),
    #[error("Model {parent} has {child} without a through table, but {child} belongs_to {child_belongs_to}")]
    BelongsToMismatch {
        parent: String,
        child: String,
        child_belongs_to: String,
    },
    #[error("Model {0} uses {1} as a through model, but {1} has no join setting")]
    MissingJoin(String, String),
    #[error(
        "Model {0} uses {1} as a through model to {2}, but {1}'s join setting does not reference {3}"
    )]
    BadJoin(String, String, String, String),
    #[error("Model {0} is a joining model and has children, which is not currently supported")]
    JoinedModelWithHas(String),
    #[error("Model {0} is a joining model, but has fields, which is not currently supported")]
    JoinedModelWithFields(String),
    #[error("Model {0}'s files configuration referenced nonexistent bucket {1}")]
    InvalidStorageBucket(String, String),
    #[error("Model {0} field {1} reference {2}")]
    FieldReferenceConfig(String, String, &'static str),
}

pub fn main() -> Result<(), Report<Error>> {
    let args = Cli::parse();

    // Commands that don't expect a config file
    match args.command {
        Command::Init(cmd) => return init::run(cmd),
        _ => {}
    };

    let config_path = args.config.clone();
    let mut config = FullConfig::from_dir(config_path)?;

    // Make sure there's a default worker config.
    if !config.config.worker.contains_key("default") {
        config.config.worker.insert(
            "default".to_string(),
            crate::config::job::WorkerConfig::default(),
        );
    }

    match args.command {
        Command::Env(cmd) => print_env::run(config, cmd),
        Command::Write(cmd) => write::write(config, cmd),
        Command::Init(_) => unreachable!(),
    }
}
