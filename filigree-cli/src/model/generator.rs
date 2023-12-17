use std::{ops::Deref, path::PathBuf};

use error_stack::{Report, ResultExt};
use rayon::prelude::*;
use serde_json::json;
use tera::Tera;

use super::{Model, SqlDialect};
use crate::{config::Config, templates::get_tera, Error, RenderedFile};

pub struct ModelGenerator<'a> {
    pub model: Model,
    pub config: &'a Config,
    pub(super) tera: &'a Tera,
    sql_context: tera::Context,
    rust_context: tera::Context,
}

impl<'a> ModelGenerator<'a> {
    pub fn new(config: &'a Config, model: Model) -> Self {
        let sql_context = Self::create_sql_context(&model, config.sql_dialect);
        let rust_context = Self::create_rust_context(&model);
        Self {
            config,
            model,
            tera: get_tera(),
            sql_context,
            rust_context,
        }
    }

    fn create_sql_context(model: &Model, dialect: SqlDialect) -> tera::Context {
        let mut context = tera::Context::new();
        context.insert("table", &model.table());
        context.insert("indexes", &model.indexes);

        context.insert("global", &model.global);
        context.insert("owner_permission", &format!("{}::owner", model.name));
        context.insert("read_permission", &format!("{}::read", model.name));
        context.insert("write_permission", &format!("{}::write", model.name));
        context.insert("extra_create_table_sql", &model.extra_create_table_sql);
        context.insert("sql_dialect", &dialect);

        let fields = model
            .all_fields()
            .map(|(fixed, field)| {
                json!({
                    "sql_name": field.sql_field_name(),
                    "sql_full_name": field.qualified_sql_field_name(),
                    "sql_type": field.typ.to_sql_type(dialect),
                    "rust_name": field.rust_field_name(),
                    "rust_type": field.rust_type.clone().unwrap_or_else(|| field.typ.to_rust_type().to_string()),
                    "default": field.default,
                    "nullable": field.nullable,
                    "unique": field.unique,
                    "extra_sql_modifiers": field.extra_sql_modifiers,
                    "user_read": field.user_access.can_read(),
                    "user_write": !fixed && field.user_access.can_write(),
                    "owner_read": field.owner_access.can_read() || field.user_access.can_read(),
                    "owner_write": !fixed && (field.owner_access.can_write() || field.user_access.can_write()),
                    "updatable": !fixed,
                })
            })
            .collect::<Vec<_>>();

        context.insert("fields", &fields);
        context
    }

    fn create_rust_context(model: &Model) -> tera::Context {
        let mut context = tera::Context::new();

        Self::add_structs_to_rust_context(model, &mut context);

        let mut extra_modules = Vec::new();

        if model.endpoints {
            extra_modules.push(json!({
                "name": "endpoints",
                "pub_use": true,
            }))
        }

        context.insert("extra_modules", &extra_modules);

        context
    }

    pub(super) fn render(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<RenderedFile, Report<Error>> {
        let output = self
            .tera
            .render(template_name, context)
            .map_err(Error::Render)
            .attach_printable_lazy(|| format!("Model {}", self.model.name))
            .attach_printable_lazy(|| format!("Template {}", template_name))?
            .into_bytes();

        let filename = template_name
            .strip_suffix(".tera")
            .expect("Template name did not end in .tera");

        let output = self
            .config
            .formatter
            .run_formatter(filename, output)
            .change_context(Error::Formatter)?;
        let path = PathBuf::from(self.model.module_name()).join(filename);

        Ok(RenderedFile {
            path,
            contents: output,
        })
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
        self.render("migrate_up.sql.tera", &self.sql_context)
            .map(|f| f.contents)
    }

    pub fn render_down_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_down.sql.tera", &self.sql_context)
            .map(|f| f.contents)
    }

    pub fn render_model_directory(&self) -> Result<Vec<RenderedFile>, Report<Error>> {
        let sql_files = [
            "select_some.sql.tera",
            "list.sql.tera",
            "insert.sql.tera",
            "update.sql.tera",
            "delete.sql.tera",
        ]
        .into_iter()
        .map(|file| (file, &self.sql_context));

        let rust_files = [
            Some("mod.rs.tera"),
            Some("types.rs.tera"),
            self.model.endpoints.then_some("endpoints.rs.tera"),
        ]
        .into_iter()
        .flatten()
        .map(|file| (file, &self.rust_context));

        let files = sql_files.chain(rust_files).collect::<Vec<_>>();

        let output = files
            .into_par_iter()
            .map(|(file, context)| self.render(file, context))
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
