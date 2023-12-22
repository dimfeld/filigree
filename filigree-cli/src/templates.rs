use std::{borrow::Cow, collections::HashMap, error::Error as _, path::Path};

use error_stack::{Report, ResultExt};
use tera::{Tera, Value};

use crate::{config::Config, Error, RenderedFile};

pub struct Renderer<'a> {
    tera: Tera,
    config: &'a Config,
}

impl<'a> Renderer<'a> {
    pub fn new(config: &'a Config) -> Self {
        let tera = create_tera();
        Self { tera, config }
    }

    pub fn render(
        &self,
        dir: &Path,
        prefix: &str,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<RenderedFile, Report<Error>> {
        let full_name = format!("{prefix}/{template_name}");
        let output = self
            .tera
            .render(&full_name, context)
            .map_err(Error::Render)
            .attach_printable_lazy(|| format!("Template {}", full_name))?
            .into_bytes();

        let filename = template_name
            .strip_suffix(".tera")
            .expect("Template name did not end in .tera");

        let output = self
            .config
            .formatter
            .run_formatter(filename, output)
            .change_context(Error::Formatter)?;
        let path = dir.join(filename);

        Ok(RenderedFile {
            path,
            contents: output,
        })
    }
}

fn create_tera() -> Tera {
    let mut tera = Tera::default();

    let res = tera.add_raw_templates(vec![
        // Model templates
        (
            "model/migrate_up.sql.tera",
            include_str!("model/sql/migrate_up.sql.tera"),
        ),
        (
            "model/migrate_down.sql.tera",
            include_str!("model/sql/migrate_down.sql.tera"),
        ),
        (
            "model/generated/delete.sql.tera",
            include_str!("model/sql/delete.sql.tera"),
        ),
        (
            "model/generated/insert.sql.tera",
            include_str!("model/sql/insert.sql.tera"),
        ),
        (
            "model/generated/list.sql.tera",
            include_str!("model/sql/list.sql.tera"),
        ),
        (
            "model/generated/select_base.sql.tera",
            include_str!("model/sql/select_base.sql.tera"),
        ),
        (
            "model/generated/select_one.sql.tera",
            include_str!("model/sql/select_one.sql.tera"),
        ),
        (
            "model/generated/select_one_all_fields.sql.tera",
            include_str!("model/sql/select_one_all_fields.sql.tera"),
        ),
        (
            "model/generated/update.sql.tera",
            include_str!("model/sql/update.sql.tera"),
        ),
        ("sql_macros.tera", include_str!("model/sql/sql_macros.tera")),
        (
            "model/mod.rs.tera",
            include_str!("model/rust_templates/mod.rs.tera"),
        ),
        (
            "model/endpoints.rs.tera",
            include_str!("model/rust_templates/endpoints.rs.tera"),
        ),
        (
            "model/generated/types.rs.tera",
            include_str!("model/rust_templates/types.rs.tera"),
        ),
        (
            "model/generated/queries.rs.tera",
            include_str!("model/rust_templates/queries.rs.tera"),
        ),
        (
            "model/generated/mod.rs.tera",
            include_str!("model/rust_templates/generated_mod.rs.tera"),
        ),
        (
            "model/main_mod.rs.tera",
            include_str!("model/rust_templates/main_mod.rs.tera"),
        ),
        // Auth templates
        (
            "auth/fetch_base.sql.tera",
            include_str!("auth/templates/fetch_base.sql.tera"),
        ),
        (
            "auth/fetch_api_key.sql.tera",
            include_str!("auth/templates/fetch_api_key.sql.tera"),
        ),
        (
            "auth/fetch_session.sql.tera",
            include_str!("auth/templates/fetch_session.sql.tera"),
        ),
    ]);

    if let Err(e) = res {
        eprintln!("{e}");
        if let Some(source) = e.source() {
            eprintln!("{source}");
        }
        panic!("Failed to add templates");
    }

    tera.register_filter("to_sql", to_sql);
    tera.register_filter("sql_string", sql_string_filter);

    tera
}

fn to_sql(val: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::String(s) => Ok(Value::String(sql_string(s))),
        Value::Array(a) => {
            let a = a
                .iter()
                .map(|s| to_sql(s, _args))
                .collect::<tera::Result<Vec<_>>>()?;
            Ok(a.into())
        }
        Value::Null => Ok(Value::String("null".to_string())),
        Value::Number(_) | Value::Bool(_) => Ok(val.clone()),
        Value::Object(_) => Err(tera::Error::msg(format!("to_sql does not support objects"))),
    }
}

fn sql_string_filter(val: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    match val {
        Value::String(s) => Ok(Value::String(sql_string(&s))),
        Value::Array(a) => {
            let a = a
                .iter()
                .map(|s| sql_string_filter(s, _args))
                .collect::<tera::Result<Vec<_>>>()?;

            Ok(a.into())
        }
        Value::Null => Ok(Value::String("null".to_string())),
        _ => Err(tera::Error::msg(format!("Value {val} is not a string"))),
    }
}

fn sql_string(s: &str) -> String {
    let inside = if s.contains('\'') {
        Cow::Owned(s.replace('\'', "''"))
    } else {
        Cow::Borrowed(s)
    };

    format!("'{inside}'")
}

#[cfg(test)]
mod test {
    use super::*;

    mod sql_string {
        use std::collections::HashMap;

        use serde_json::json;

        use super::*;

        fn call(val: impl Into<serde_json::Value>) -> String {
            sql_string_filter(&val.into(), &HashMap::default())
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        }

        #[test]
        fn no_quotes_string() {
            assert_eq!(call("hello".to_string()), "'hello'");
        }

        #[test]
        fn quotes_string() {
            assert_eq!(call("don't".to_string()), "'don''t");
        }

        #[test]
        fn array() {
            assert_eq!(
                call(json!(["hello", "wo'rld"])),
                json!(["'hello'", "'wo''rld'"])
            );
        }
    }
}
