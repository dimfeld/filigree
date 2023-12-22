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

mod auth;
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

    /// Force regenerating all templates, even those which would normally be generated
    /// only once.
    /// (Eventually these files and the always-regenerated files will be generated as separate
    /// commands.)
    #[clap(long)]
    force: bool,
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

    let renderer = templates::Renderer::new(&config);

    let models = build_models(config_models);

    let generators = models
        .into_iter()
        .map(|model| ModelGenerator::new(&config, &renderer, model))
        .collect::<Vec<_>>();
    let all_model_contexts = generators
        .iter()
        .map(|m| (m.model.name.clone(), m.context.clone().into_json()))
        .collect::<Vec<_>>();

    let model_files = generators
        .par_iter()
        .map(|gen| gen.render_model_directory())
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
    let (before_up, after_up) = ModelGenerator::fixed_up_migration_files();
    write_vecs(
        &up_migration_path,
        args.force,
        before_up,
        &up_migrations,
        after_up,
        b"\n\n",
    )
    .change_context(Error::WriteFile)?;

    let down_migration_path = config
        .migrations_path
        .join("00000000000000_filigree_init.down.sql");
    let (before_down, after_down) = ModelGenerator::fixed_down_migration_files();
    write_vecs(
        &down_migration_path,
        args.force,
        before_down,
        &down_migrations,
        after_down,
        b"\n\n",
    )
    .change_context(Error::WriteFile)?;

    let files = model_files.into_iter().flatten().collect::<Vec<_>>();

    let mut created_dirs = HashSet::new();
    for file in &files {
        let parent = file.path.parent();
        if let Some(parent) = parent {
            let dir = config.models_path.join(parent);
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
            let path = config.models_path.join(&file.path);
            if !args.force && !path.to_string_lossy().contains("/generated/") && path.exists() {
                return Ok(());
            }

            // eprintln!("Writing file {}", path.display());
            std::fs::write(&path, &file.contents)
                .attach_printable_lazy(|| path.display().to_string())
        })
        .change_context(Error::WriteFile)?;

    let mut model_mod_context = tera::Context::new();
    model_mod_context.insert(
        "models",
        &all_model_contexts
            .iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>(),
    );

    let model_mod = renderer.render(
        &config.models_path,
        "model",
        "main_mod.rs.tera",
        &model_mod_context,
    )?;
    let path = config.models_path.join("mod.rs");
    if args.force || !path.exists() {
        std::fs::write(&path, model_mod.contents)
            .attach_printable_lazy(|| path.display().to_string())
            .change_context(Error::WriteFile)?;
    }

    Ok(())
}

fn write_vecs(
    path: &Path,
    overwrite: bool,
    before: String,
    data: &[Vec<u8>],
    after: String,
    sep: &[u8],
) -> Result<(), std::io::Error> {
    if !overwrite && path.exists() {
        return Ok(());
    }

    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(before.as_bytes())?;

    for item in data.iter() {
        writer.write_all(sep)?;
        writer.write_all(item)?;
    }

    writer.write_all(sep)?;
    writer.write_all(after.as_bytes())?;

    writer.flush()?;
    Ok(())
}
