use std::{
    collections::HashSet,
    error::Error as _,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use config::Config;
use error_stack::{Report, ResultExt};
use model::Model;
use rayon::prelude::*;
use thiserror::Error;

use crate::{config::FullConfig, model::generator::ModelGenerator};

mod add_deps;
mod auth;
mod config;
mod format;
mod model;
mod root;
mod server;
mod templates;
mod users;

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
    #[error("Failed to run cargo")]
    Cargo,
}

fn build_models(config: &Config, mut config_models: Vec<Model>) -> Vec<Model> {
    let mut models = Model::create_default_models(config);
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

    let config_path = args.config.clone();
    let config = FullConfig::from_dir(config_path)?;

    let FullConfig {
        crate_name,
        config,
        models: config_models,
        crate_manifest,
    } = config;

    add_deps::add_deps(&crate_manifest)?;

    let renderer = templates::Renderer::new(&config);

    let models = build_models(&config, config_models);

    let generators = models
        .into_iter()
        .map(|model| ModelGenerator::new(&config, &renderer, model))
        .collect::<Vec<_>>();
    let all_model_contexts = generators
        .iter()
        .map(|m| (m.model.name.clone(), m.context.clone().into_json()))
        .collect::<Vec<_>>();

    let mut auth_files = None;
    let mut model_files = None;
    let mut root_files = None;
    let mut server_files = None;
    let mut users_files = None;
    rayon::scope(|s| {
        s.spawn(|_| {
            auth_files = Some(auth::render_files(&config, &renderer, &all_model_contexts));
        });
        s.spawn(|_| {
            model_files = Some(
                generators
                    .par_iter()
                    .map(|gen| gen.render_model_directory())
                    .collect::<Result<Vec<_>, _>>(),
            );
        });
        s.spawn(|_| root_files = Some(root::render_files(&crate_name, &config, &renderer)));
        s.spawn(|_| server_files = Some(server::render_files(&config, &renderer)));
        s.spawn(|_| {
            users_files = Some(users::render_files(&config, &renderer, &all_model_contexts));
        });
    });

    let auth_files = auth_files.expect("auth_files was not set")?;
    let model_files = model_files.expect("model_files was not set")?;
    let root_files = root_files.expect("root_files was not set")?;
    let server_files = server_files.expect("server_files was not set")?;
    let users_files = users_files.expect("users_files was not set")?;

    let up_migrations = generators
        .iter()
        .map(|gen| gen.render_up_migration())
        .collect::<Result<Vec<_>, _>>()?;

    let down_migrations = generators
        .iter()
        .map(|gen| gen.render_down_migration())
        .collect::<Result<Vec<_>, _>>()?;

    let migrations_dir = PathBuf::from("migrations");

    std::fs::create_dir_all(&migrations_dir)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| {
            format!(
                "Unable to create migrations directory {0}",
                migrations_dir.display()
            )
        })?;

    // TODO This works once but we don't want to change the initial migration after it's been
    // created.
    let up_migration_path = migrations_dir.join("00000000000000_filigree_init.up.sql");
    let (before_up, after_up) = ModelGenerator::fixed_up_migration_files();
    write_vecs(
        &up_migration_path,
        before_up,
        &up_migrations,
        after_up,
        b"\n\n",
    )
    .change_context(Error::WriteFile)?;

    let down_migration_path = migrations_dir.join("00000000000000_filigree_init.down.sql");
    let (before_down, after_down) = ModelGenerator::fixed_down_migration_files();
    write_vecs(
        &down_migration_path,
        before_down,
        &down_migrations,
        after_down,
        b"\n\n",
    )
    .change_context(Error::WriteFile)?;

    let files = root_files
        .into_iter()
        .chain(auth_files)
        .chain(model_files.into_iter().flatten())
        .chain(server_files)
        .chain(users_files)
        .collect::<Vec<_>>();

    let mut created_dirs = HashSet::new();
    for file in &files {
        let parent = file.path.parent();
        if let Some(dir) = parent {
            if !created_dirs.contains(&dir) {
                std::fs::create_dir_all(&dir)
                    .change_context(Error::WriteFile)
                    .attach_printable_lazy(|| {
                        format!("Unable to create directory {}", dir.display())
                    })?;
                created_dirs.insert(dir);
            }
        }
    }

    files
        .into_par_iter()
        .try_for_each(|file| {
            // eprintln!("Writing file {}", path.display());
            std::fs::write(&file.path, &file.contents)
                .attach_printable_lazy(|| file.path.display().to_string())
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

    let models_main_mod_path = PathBuf::from("src/models/mod.rs");
    let model_mod = renderer.render_with_full_path(
        models_main_mod_path,
        "model/main_mod.rs.tera",
        &model_mod_context,
    )?;
    std::fs::write(&model_mod.path, model_mod.contents)
        .attach_printable_lazy(|| model_mod.path.display().to_string())
        .change_context(Error::WriteFile)?;

    Ok(())
}

fn write_vecs(
    path: &Path,
    before: String,
    data: &[Vec<u8>],
    after: String,
    sep: &[u8],
) -> Result<(), std::io::Error> {
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
