use std::{collections::HashSet, error::Error as _, path::PathBuf};

use clap::Parser;
use config::Config;
use error_stack::{Report, ResultExt};
use merge_files::MergeTracker;
use migrations::{resolve_migration, save_migration_state, SingleMigration};
use model::Model;
use rayon::prelude::*;
use thiserror::Error;

use crate::{config::FullConfig, model::generator::ModelGenerator};

mod add_deps;
mod config;
mod format;
mod merge_files;
mod migrations;
mod model;
mod root;
mod state;
mod templates;

pub enum RenderedFileLocation {
    Api,
    Web,
}

pub struct RenderedFile {
    path: PathBuf,
    contents: Vec<u8>,
    location: RenderedFileLocation,
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
    #[error("Failed to read migration files")]
    ReadMigrationFiles,
    #[error("Failed to write file")]
    WriteFile,
    #[error("{0}{}", .0.source().map(|e| format!("\n{e}")).unwrap_or_default())]
    Render(#[from] tera::Error),
    #[error("Failed to run code formatter")]
    Formatter,
    #[error("Failed to run cargo")]
    Cargo,
    #[error("Input error")]
    Input,
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
        api_dir,
        web_dir,
        state,
    } = config;
    let api_merge_tracker = MergeTracker::new(state_dir.join("api"), api_dir.clone());
    let web_merge_tracker = MergeTracker::new(state_dir.join("web"), web_dir.clone());

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

    let model_migrations = generators
        .iter()
        .map(|gen| {
            let up = gen.render_up_migration()?;
            let down = gen.render_down_migration()?;
            let result = SingleMigration {
                up: String::from_utf8(up).unwrap().into(),
                down: String::from_utf8(down).unwrap().into(),
                model: Some(&gen.model),
                name: gen.model.table(),
            };

            Ok::<_, Report<Error>>(result)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (first_fixed_migrations, last_fixed_migrations) = ModelGenerator::fixed_migrations();

    let migrations = first_fixed_migrations
        .into_iter()
        .chain(model_migrations.into_iter())
        .chain(last_fixed_migrations.into_iter())
        .collect::<Vec<_>>();

    let migrations_dir = api_dir.join("migrations");

    std::fs::create_dir_all(&migrations_dir)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| {
            format!(
                "Unable to create migrations directory {0}",
                migrations_dir.display()
            )
        })?;

    let migration = resolve_migration(&migrations_dir, &state_dir, &migrations)?;

    if !migration.up.is_empty() {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

        let migration_name = dialoguer::Input::<String>::new()
            .with_prompt("Enter a name for the migration")
            .interact_text()
            .change_context(Error::Input)?;

        let migration_name = migration_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        let up_filename = format!("{timestamp}_{migration_name}.up.sql");
        let down_filename = format!("{timestamp}_{migration_name}.down.sql");

        let up = config
            .formatter
            .run_formatter(&up_filename, migration.up.into_owned().into_bytes())?;
        let down = config
            .formatter
            .run_formatter(&down_filename, migration.down.into_owned().into_bytes())?;

        std::fs::write(migrations_dir.join(&up_filename), &up)
            .change_context(Error::WriteFile)
            .attach_printable(up_filename)?;
        std::fs::write(migrations_dir.join(&down_filename), &down)
            .change_context(Error::WriteFile)
            .attach_printable(down_filename)?;
    }

    save_migration_state(&state_dir, &migrations)?;

    let (api_files, web_files): (Vec<_>, Vec<_>) = root_files
        .into_iter()
        .chain(model_files.into_iter().flatten())
        .partition(|f| matches!(f.location, RenderedFileLocation::Api));

    let filesets = [
        (api_files, &api_dir, &api_merge_tracker),
        (web_files, &web_dir, &web_merge_tracker),
    ];

    let merge_files = filesets
        .into_par_iter()
        .map(|(files, base_dir, merge_tracker)| {
            let mut created_dirs = HashSet::new();
            for file in &files {
                let parent = file.path.parent();
                if let Some(dir) = parent {
                    if !created_dirs.contains(&dir) {
                        std::fs::create_dir_all(&base_dir.join(dir))
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

            Ok::<_, Report<Error>>(merge_files)
        })
        .collect::<Result<Vec<_>, _>>()?;

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
        RenderedFileLocation::Api,
        &model_mod_context,
    )?;

    let models_output = api_merge_tracker.from_rendered_file(model_mod);

    let merge_files = merge_files
        .into_iter()
        .flatten()
        .chain([models_output])
        .collect::<Vec<_>>();

    let mut conflict_files = merge_files
        .iter()
        .filter(|f| f.merged.conflicts)
        .map(|f| f.output_path.clone())
        .collect::<Vec<_>>();
    conflict_files.sort();

    merge_files
        .into_par_iter()
        .try_for_each(|file| file.write())
        .change_context(Error::WriteFile)?;

    if !conflict_files.is_empty() {
        println!("=== Files with conflicts");

        for path in conflict_files {
            println!("{}", path.display());
        }
    }

    Ok(())
}
