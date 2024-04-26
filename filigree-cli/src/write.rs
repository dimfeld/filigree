use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use clap::Args;
use error_stack::{Report, ResultExt};
use itertools::Itertools;
use rayon::prelude::*;

use crate::{
    config::{Config, FullConfig},
    format::Formatters,
    merge_files::MergeTracker,
    migrations::{resolve_migration, save_migration_state, SingleMigration},
    model::{generator::ModelGenerator, Model},
    Error,
};

#[derive(Args, Debug)]
pub struct Command {
    /// When generating files, ignore any changes you have made and just write the template output,
    /// instead of merging them together.
    #[clap(long)]
    overwrite: bool,
}

pub enum RenderedFileLocation {
    Rust,
    Svelte,
    Htmx,
}

pub struct RenderedFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
    pub location: RenderedFileLocation,
}

pub struct ModelMap(pub std::collections::HashMap<String, Model>);

impl ModelMap {
    pub fn new(models: &[Model]) -> Self {
        let model_map = models
            .iter()
            .cloned()
            .map(|m| (m.name.clone(), m))
            .collect();

        Self(model_map)
    }

    pub fn get(&self, name: &str, from_model: &str, context: &str) -> Result<&Model, Error> {
        self.0.get(name).ok_or_else(|| {
            Error::MissingModel(
                name.to_string(),
                from_model.to_string(),
                context.to_string(),
            )
        })
    }
}

pub struct GeneratorMap<'a>(pub std::collections::HashMap<String, &'a ModelGenerator<'a>>);

impl<'a> GeneratorMap<'a> {
    pub fn new(models: &'a [ModelGenerator]) -> Self {
        let model_map = models.iter().map(|m| (m.model.name.clone(), m)).collect();

        Self(model_map)
    }

    pub fn get(
        &self,
        name: &str,
        from_model: &str,
        context: &str,
    ) -> Result<&ModelGenerator, Error> {
        self.0.get(name).map(|g| *g).ok_or_else(|| {
            Error::MissingModel(
                name.to_string(),
                from_model.to_string(),
                context.to_string(),
            )
        })
    }
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

    // Add autogenerated submodels for file uploads
    for model in config_models.iter() {
        for file in &model.files {
            models.push(file.generate_model(&model));
        }
    }

    models.extend(config_models.into_iter());

    models
}

pub fn write(config: FullConfig, args: Command) -> Result<(), Report<Error>> {
    let web_relative_to_api = config.web_relative_to_api();
    let FullConfig {
        crate_name,
        config,
        models: config_models,
        mut crate_manifest,
        state_dir,
        api_dir,
        web_dir,
        pages,
        ..
    } = config;

    let formatter = Formatters {
        config: config.formatter.clone(),
        api_base_dir: api_dir.clone(),
        web_base_dir: web_dir.clone(),
    };

    let api_merge_tracker =
        MergeTracker::new(state_dir.join("api"), api_dir.clone(), args.overwrite);
    let web_merge_tracker =
        MergeTracker::new(state_dir.join("web"), web_dir.clone(), args.overwrite);

    let renderer = crate::templates::Renderer::new(formatter.clone());

    let models = build_models(&config, config_models);
    let model_map = ModelMap::new(&models);

    crate::model::validate::validate_model_configuration(&config, &model_map)?;

    crate::add_deps::add_fixed_deps(&api_dir, &config, &mut crate_manifest)?;
    config.web.add_deps(&api_dir, &mut crate_manifest)?;
    for model in &models {
        model.add_deps(&api_dir, &mut crate_manifest)?;
    }

    let mut generators = models
        .into_iter()
        .map(|model| ModelGenerator::new(&config, &renderer, &model_map, model))
        .collect::<Result<Vec<_>, Error>>()?;
    generators.sort_by(|a, b| a.model.order_by_dependency(&b.model));

    let generator_map = GeneratorMap::new(&generators);

    // The generators may need references to each other, so we can only create the template context
    // once they all exist.
    let generator_contexts = generators
        .iter()
        .map(|g| {
            Ok((
                g.model.name.clone(),
                g.create_template_context(&generator_map)?,
            ))
        })
        .collect::<Result<HashMap<_, _>, Error>>()?;

    for g in &mut generators {
        g.set_template_context(generator_contexts[&g.model.name].clone())
    }

    let mut model_files = None;
    let mut root_files = None;
    let mut page_files = None;
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
            root_files = Some(crate::root::render_files(
                &crate_name,
                &config,
                web_relative_to_api,
                &generators,
                &renderer,
            ))
        });

        if config.web.has_api_pages() {
            s.spawn(|_| page_files = Some(crate::root::pages::render_pages(pages, &renderer)));
        }
    });

    let model_files = model_files.expect("model_files was not set")?;
    let root_files = root_files.expect("root_files was not set")?;
    let page_files = page_files.unwrap_or(Ok(Vec::new()))?;

    let mut model_migrations = generators
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

    // When a child model belongs to a parent model, ensure that the child comes later.
    model_migrations.sort_by(|m1, m2| {
        let m1 = &m1.model.unwrap();
        let m2 = &m2.model.unwrap();
        // The normal ordering places child tables first, so reverse it here. For migrations we want the child table to
        // come second because the foreign key constraint is on the child table so the parent must
        // be created first.
        m1.order_by_dependency(m2).reverse()
    });

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

        let up = formatter.run_formatter(&up_filename, migration.up.into_owned().into_bytes())?;
        let down =
            formatter.run_formatter(&down_filename, migration.down.into_owned().into_bytes())?;

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
        .chain(page_files.into_iter())
        .chain(model_files.into_iter().flatten())
        .partition(|f| matches!(f.location, RenderedFileLocation::Rust));

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

            let mut merge_files = files
                .into_par_iter()
                .map(|f| merge_tracker.from_rendered_file(f))
                .collect::<Vec<_>>();

            let always_keep = if base_dir == &api_dir {
                vec!["src/models/mod.rs"]
            } else {
                vec![]
            };

            let empty_files = merge_tracker.generate_empty_files(&merge_files, &always_keep);
            merge_files.extend(empty_files);

            Ok::<_, Report<Error>>(merge_files)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut model_mod_context = tera::Context::new();
    model_mod_context.insert(
        "models",
        &generators
            .iter()
            .sorted_by(|m1, m2| m1.model.name.cmp(&m2.model.name))
            .map(|v| v.template_context().clone().into_json())
            .collect::<Vec<_>>(),
    );

    let models_main_mod_path = PathBuf::from("src/models/mod.rs");
    let model_mod = renderer.render_with_full_path(
        models_main_mod_path,
        "model/main_mod.rs.tera",
        RenderedFileLocation::Rust,
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
