use std::{ops::Deref, path::PathBuf};

use error_stack::{Report, ResultExt};
use rayon::prelude::*;

use super::Model;
use crate::{
    config::Config,
    templates::{ModelRustTemplates, ModelSqlTemplates, Renderer},
    Error, RenderedFile,
};

pub struct ModelGenerator<'a> {
    pub model: Model,
    pub(super) renderer: &'a Renderer<'a>,
    pub config: &'a Config,
    pub context: tera::Context,
}

impl<'a> ModelGenerator<'a> {
    pub fn new(config: &'a Config, renderer: &'a Renderer<'a>, model: Model) -> Self {
        let context = Self::create_template_context(&config, &model);
        Self {
            config,
            model,
            renderer,
            context,
        }
    }

    fn create_template_context(config: &Config, model: &Model) -> tera::Context {
        let sql_dialect = Config::default_sql_dialect();
        let mut context = tera::Context::from_value(model.template_context(config)).unwrap();
        let base_dir = PathBuf::from("src/models").join(model.module_name());
        context.insert("dir", &base_dir);
        context.insert("module_name", &model.module_name());

        context.insert("model_name", &model.name);
        context.insert("sql_dialect", &sql_dialect);

        Self::add_structs_to_rust_context(model, &mut context);

        context
    }

    pub fn fixed_up_migration_files() -> (String, String) {
        let before_up = [include_str!("../../sql/delete_log.up.sql")].join("\n\n");

        let after_up = [
            include_str!("../../sql/user_info.up.sql"),
            include_str!("../../sql/create_permissions.up.sql"),
            include_str!("../../sql/create_object_permissions.up.sql"),
        ]
        .join("\n\n");

        (before_up, after_up)
    }

    pub fn fixed_down_migration_files() -> (String, String) {
        let before_down = [include_str!("../../sql/delete_log.down.sql")].join("\n\n");
        let after_down = [
            include_str!("../../sql/user_info.down.sql"),
            include_str!("../../sql/create_permissions.down.sql"),
            include_str!("../../sql/create_object_permissions.down.sql"),
        ]
        .join("\n\n");

        (before_down, after_down)
    }

    pub fn render_up_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.renderer
            .render(&PathBuf::new(), "model/migrate_up.sql.tera", &self.context)
            .map(|f| f.contents)
    }

    pub fn render_down_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.renderer
            .render(
                &PathBuf::new(),
                "model/migrate_down.sql.tera",
                &self.context,
            )
            .map(|f| f.contents)
    }

    pub fn render_model_directory(&self) -> Result<Vec<RenderedFile>, Report<Error>> {
        let base_path = PathBuf::from("src/models").join(self.model.module_name());

        let skip_files = [
            "model/main_mod.rs.tera",
            "model/sql_macros.tera",
            "model/migrate_up.sql.tera",
            "model/migrate_down.sql.tera",
            "model/select_base.sql.tera",
        ];

        let files = ModelSqlTemplates::iter()
            .chain(ModelRustTemplates::iter())
            .filter(|f| !skip_files.contains(&f.as_ref()))
            .collect::<Vec<_>>();

        let output = files
            .into_par_iter()
            .map(|file| {
                let filename = file
                    .strip_prefix("model/")
                    .unwrap()
                    .strip_suffix(".tera")
                    .unwrap();
                let path = base_path.join(filename);
                self.renderer
                    .render_with_full_path(path, &file, &self.context)
                    .attach_printable_lazy(|| format!("Model {}", self.model.name))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(output)
    }
}

impl Deref for ModelGenerator<'_> {
    type Target = Model;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}
