use std::{ops::Deref, path::PathBuf};

use error_stack::{Report, ResultExt};
use rayon::prelude::*;
use serde_json::json;

use super::Model;
use crate::{config::Config, templates::Renderer, Error, RenderedFile};

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
        let mut context = tera::Context::new();
        context.insert("dir", &config.models_path.join(model.module_name()));
        context.insert("module_name", &model.module_name());

        context.insert("table", &model.table());
        context.insert("indexes", &model.indexes);

        context.insert("global", &model.global);
        context.insert("owner_permission", &format!("{}::owner", model.name));
        context.insert("read_permission", &format!("{}::read", model.name));
        context.insert("write_permission", &format!("{}::write", model.name));
        context.insert("extra_create_table_sql", &model.extra_create_table_sql);
        context.insert("sql_dialect", &config.sql_dialect);
        context.insert("pagination", &model.pagination);

        let fields = model
            .all_fields()
            .map(|(fixed, field)| {
                json!({
                    "sql_name": field.sql_field_name(),
                    "sql_full_name": field.qualified_sql_field_name(),
                    "sql_type": field.typ.to_sql_type(config.sql_dialect),
                    "rust_name": field.rust_field_name(),
                    "base_rust_type": field.base_rust_type(),
                    "rust_type": field.rust_type(),
                    "default": field.default,
                    "nullable": field.nullable,
                    "filterable": field.filterable,
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

        Self::add_structs_to_rust_context(model, &mut context);

        let mut extra_modules = Vec::new();

        context.insert("id_type", &model.object_id_type());
        context.insert("id_prefix", &model.id_prefix());

        let module_name = model.module_name();
        context.insert(
            "predefined_object_id",
            &["role", "user", "organization"].contains(&module_name.as_str()),
        );
        context.insert("url_path", &module_name);
        context.insert("has_any_endpoints", &model.endpoints.any_enabled());
        context.insert("endpoints", &model.endpoints.per_endpoint());
        if model.endpoints.any_enabled() {
            extra_modules.push(json!({
                "name": "endpoints",
                "pub_use": true,
            }))
        }

        context.insert("extra_modules", &extra_modules);

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

    pub fn render(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<RenderedFile, Report<Error>> {
        self.renderer
            .render(
                &PathBuf::from(self.model.module_name()),
                "model",
                template_name,
                context,
            )
            .attach_printable_lazy(|| format!("Model {}", self.model.name))
    }

    pub fn render_up_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_up.sql.tera", &self.context)
            .map(|f| f.contents)
    }

    pub fn render_down_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.render("migrate_down.sql.tera", &self.context)
            .map(|f| f.contents)
    }

    pub fn render_model_directory(&self) -> Result<Vec<RenderedFile>, Report<Error>> {
        let files = [
            Some("select_some.sql.tera"),
            Some("list.sql.tera"),
            Some("insert.sql.tera"),
            Some("update.sql.tera"),
            Some("delete.sql.tera"),
            Some("mod.rs.tera"),
            Some("types.rs.tera"),
            Some("queries.rs.tera"),
            self.model
                .endpoints
                .any_enabled()
                .then_some("endpoints.rs.tera"),
        ];

        let output = files
            .into_par_iter()
            .flatten()
            .map(|file| self.render(file, &self.context))
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
