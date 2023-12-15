use std::ops::Deref;

use error_stack::{Report, ResultExt};
use serde_json::json;
use tera::Tera;

use super::{Model, SqlDialect};
use crate::{config::Config, format::run_formatter, templates::get_tera, Error};

pub struct ModelGenerator<'a> {
    pub model: Model,
    pub config: &'a Config,
    pub(super) context: tera::Context,
    pub(super) tera: &'a Tera,
}

impl<'a> ModelGenerator<'a> {
    pub fn new(config: &'a Config, model: Model) -> Self {
        Self {
            context: Self::create_template_context(&model, config.sql_dialect),
            config,
            model,
            tera: get_tera(),
        }
    }

    fn create_template_context(model: &Model, dialect: SqlDialect) -> tera::Context {
        let mut context = tera::Context::new();
        context.insert("table", &model.table());
        context.insert("indexes", &model.indexes);

        context.insert("global", &model.global);
        context.insert("owner_permission", &format!("{}::owner", model.id_prefix));
        context.insert("read_permission", &format!("{}::read", model.name));
        context.insert("write_permission", &format!("{}::write", model.id_prefix));
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

    pub(super) fn render<'f>(
        &self,
        template_name: &'f str,
    ) -> Result<(&'f str, Vec<u8>), Report<Error>> {
        let output = self
            .tera
            .render(template_name, &self.context)
            .change_context(Error::Render)
            .attach_printable_lazy(|| format!("Model {}", self.model.name))
            .attach_printable_lazy(|| format!("Template {}", template_name))?
            .into_bytes();

        let filename = template_name
            .strip_suffix(".tera")
            .expect("Template name did not end in .tera");

        let formatter = if filename.ends_with(".sql") {
            self.config.formatter.sql.as_ref()
        } else if filename.ends_with(".rs") {
            self.config.formatter.rust.as_ref()
        } else {
            None
        };

        let output = if let Some(formatter) = formatter {
            run_formatter(formatter, output).change_context(Error::Formatter)?
        } else {
            output
        };

        Ok((filename, output))
    }

    pub(super) fn write_to_file(&self, filename: &str, output: &[u8]) -> Result<(), Report<Error>> {
        let path = self
            .config
            .generated_path
            .join(self.model.module_name())
            .join(filename);

        std::fs::write(&path, output)
            .change_context(Error::WriteFile)
            .attach_printable_lazy(|| path.display().to_string())?;
        Ok(())
    }
}

impl Deref for ModelGenerator<'_> {
    type Target = Model;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}
