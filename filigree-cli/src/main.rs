use std::{
    collections::HashSet,
    error::Error as _,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use error_stack::{Report, ResultExt};
use model::Model;
use rayon::prelude::*;
use thiserror::Error;

use crate::{config::FullConfig, model::generator::ModelGenerator};

pub mod config;
mod format;
pub mod model;
pub mod templates;

pub struct RenderedFile {
    path: PathBuf,
    contents: Vec<u8>,
}

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
    #[error("{0}{}", .0.source().map(|e| format!("\n{e}")).unwrap_or_default())]
    Render(#[from] tera::Error),
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

    let sql_files = generators
        .par_iter()
        .map(|gen| gen.render_sql_queries())
        .collect::<Result<Vec<_>, _>>()?;

    let up_migrations = generators
        .iter()
        .map(|gen| gen.render_up_migration())
        .collect::<Result<Vec<_>, _>>()?;

    let down_migrations = generators
        .iter()
        .map(|gen| gen.render_down_migration())
        .collect::<Result<Vec<_>, _>>()?;

    std::fs::create_dir_all(&config.migrations_path)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| {
            format!(
                "Unable to create migrations directory {}",
                config.migrations_path.display()
            )
        })?;

    // TODO This works once but we don't want to change the initial migration after it's been
    // created.
    let up_migration_path = config
        .migrations_path
        .join("00000000000000_filigree_init.up.sql");
    write_vecs(&up_migration_path, &up_migrations, b"\n\n").change_context(Error::WriteFile)?;
    let down_migration_path = config
        .migrations_path
        .join("00000000000000_filigree_init.down.sql");
    write_vecs(&down_migration_path, &down_migrations, b"\n\n").change_context(Error::WriteFile)?;

    let files = sql_files.into_iter().flatten().collect::<Vec<_>>();

    let mut created_dirs = HashSet::new();
    for file in &files {
        let parent = file.path.parent();
        if let Some(parent) = parent {
            let dir = config.generated_path.join(parent);
            if !created_dirs.contains(&dir) {
                std::fs::create_dir_all(&dir)
                    .change_context(Error::WriteFile)
                    .attach_printable_lazy(|| {
                        format!("Unable to create directory {}", parent.display())
                    })?;
                created_dirs.insert(dir);
            }
        }
    }

    files
        .into_par_iter()
        .try_for_each(|file| {
            let path = config.generated_path.join(&file.path);
            eprintln!("Writing file {}", path.display());
            std::fs::write(&path, &file.contents)
                .attach_printable_lazy(|| path.display().to_string())
        })
        .change_context(Error::WriteFile)?;

    Ok(())
}

fn write_vecs(path: &Path, data: &[Vec<u8>], sep: &[u8]) -> Result<(), std::io::Error> {
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);

    for (i, item) in data.iter().enumerate() {
        if i > 0 {
            writer.write_all(sep)?;
        }
        writer.write_all(item)?;
    }

    writer.flush()?;
    Ok(())
}
