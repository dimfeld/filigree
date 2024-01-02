use std::{ops::Deref, path::PathBuf};

use convert_case::{Case, Casing};
use error_stack::{Report, ResultExt};
use rayon::prelude::*;
use serde_json::json;

use super::{field::SortableType, Model};
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
        let mut context = tera::Context::new();
        let base_dir = PathBuf::from("src/models").join(model.module_name());
        context.insert("dir", &base_dir);
        context.insert("module_name", &model.module_name());

        context.insert("model_name", &model.name);
        context.insert("table", &model.table());
        context.insert("indexes", &model.indexes);

        context.insert("global", &model.global);
        context.insert("owner_permission", &format!("{}::owner", model.name));
        context.insert("read_permission", &format!("{}::read", model.name));
        context.insert("write_permission", &format!("{}::write", model.name));
        context.insert("extra_create_table_sql", &model.extra_create_table_sql);
        context.insert("sql_dialect", &sql_dialect);
        context.insert("pagination", &model.pagination);
        context.insert(
            "auth_scope",
            &model.auth_scope.unwrap_or(config.default_auth_scope),
        );

        let full_default_sort_field = model.default_sort_field.as_deref().unwrap_or("-updated_at");
        let default_sort_field = if full_default_sort_field.starts_with('-') {
            &full_default_sort_field[1..]
        } else {
            full_default_sort_field
        };

        {
            let default_field = model
                .all_fields()
                .find(|(_, f)| f.name == default_sort_field);
            if let Some(default_field) = default_field {
                if default_field.1.sortable == SortableType::None {
                    panic!(
                        "Model {}: Default sort field {default_sort_field} is not sortable",
                        model.name
                    );
                }
            } else {
                panic!(
                    "Model {}, Default sort field {} does not exist in model {}",
                    model.name, default_sort_field, model.name
                );
            }
        }

        context.insert("full_default_sort_field", full_default_sort_field);
        context.insert("default_sort_field", default_sort_field);

        let fields = model
            .all_fields()
            .map(|(fixed, field)| {
                json!({
                    "name": field.name,
                    "sql_name": field.sql_field_name(),
                    "sql_full_name": field.qualified_sql_field_name(),
                    "sql_type": field.typ.to_sql_type(sql_dialect),
                    "snake_case_name": field.name.to_case(Case::Snake),
                    "pascal_case_name": field.name.to_case(Case::Pascal),
                    "rust_name": field.rust_field_name(),
                    "base_rust_type": field.base_rust_type(),
                    "rust_type": field.rust_type(),
                    "is_custom_rust_type": field.rust_type.is_some(),
                    "default_sql": field.default_sql,
                    "default_rust": field.default_rust,
                    "nullable": field.nullable,
                    "filterable": field.filterable,
                    "sortable": field.sortable,
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
