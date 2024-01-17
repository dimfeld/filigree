use std::{
    collections::HashSet,
    error::Error as _,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use config::Config;
use error_stack::{Report, ResultExt};
use merge_files::MergeTracker;
use model::Model;
use rayon::prelude::*;
use thiserror::Error;

use crate::{config::FullConfig, model::generator::ModelGenerator};

mod add_deps;
mod config;
mod format;
mod merge_files;
mod model;
mod root;
mod state;
mod templates;

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
        state_dir,
        crate_base_dir,
        state,
    } = config;
    let merge_tracker = MergeTracker::new(state_dir.clone(), crate_base_dir.clone());

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

    let mut model_files = None;
    let mut root_files = None;
    rayon::scope(|s| {
        s.spawn(|_| {
            model_files = Some(
                generators
                    .par_iter()
                    .map(|gen| gen.render_model_directory())
                    .collect::<Result<Vec<_>, _>>(),
            );
        });
        s.spawn(|_| {
            root_files = Some(root::render_files(
                &crate_name,
                &config,
                &all_model_contexts,
                &renderer,
            ))
        });
    });

    let model_files = model_files.expect("model_files was not set")?;
    let root_files = root_files.expect("root_files was not set")?;

    let up_migrations = generators
        .iter()
        .map(|gen| gen.render_up_migration())
        .collect::<Result<Vec<_>, _>>()?;

    let down_migrations = generators
        .iter()
        .map(|gen| gen.render_down_migration())
        .collect::<Result<Vec<_>, _>>()?;

    let migrations_dir = crate_base_dir.join("migrations");

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
        .chain(model_files.into_iter().flatten())
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

                let gen_cache_dir = merge_tracker.base_generated_path.join(&dir);

                std::fs::create_dir_all(&gen_cache_dir)
                    .change_context(Error::WriteFile)
                    .attach_printable_lazy(|| {
                        format!("Unable to create directory {}", dir.display())
                    })?;
            }
        }
    }

    let merge_files = files
        .into_par_iter()
        .map(|f| merge_tracker.from_rendered_file(f))
        .collect::<Vec<_>>();

    let mut conflict_files = merge_files
        .iter()
        .filter(|f| f.merged.conflicts)
        .map(|f| &f.output_path)
        .collect::<Vec<_>>();
    conflict_files.sort();

    if !conflict_files.is_empty() {
        println!("=== Files with conflicts");

        for path in conflict_files {
            println!("{}", path.display());
        }
    }

    merge_files
        .into_par_iter()
        .try_for_each(|file| file.write())
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

    let models_output = merge_tracker.from_rendered_file(model_mod);
    models_output.write().change_context(Error::WriteFile)?;

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
