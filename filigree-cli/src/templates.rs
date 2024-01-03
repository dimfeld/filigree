use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error as _,
    path::{Path, PathBuf},
};

use error_stack::{Report, ResultExt};
use rust_embed::RustEmbed;
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

    /// Render a template, joining the template name to `dir` to calculate the output path.
    pub fn render(
        &self,
        dir: &Path,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<RenderedFile, Report<Error>> {
        let path = dir.join(
            template_name
                .strip_suffix(".tera")
                .expect("template name did not end with .tera"),
        );
        self.render_with_full_path(path, template_name, context)
    }

    /// Render a template with a precalculated path
    pub fn render_with_full_path(
        &self,
        path: PathBuf,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<RenderedFile, Report<Error>> {
        let output = self
            .tera
            .render(template_name, context)
            .map_err(Error::Render)
            .attach_printable_lazy(|| format!("Template {}", template_name))?
            .into_bytes();

        let filename = template_name.strip_suffix(".tera").unwrap_or(template_name);

        let output = self
            .config
            .formatter
            .run_formatter(filename, output)
            .change_context(Error::Formatter)?;

        Ok(RenderedFile {
            path,
            contents: output,
        })
    }
}

#[derive(RustEmbed)]
#[prefix = "root/"]
#[folder = "$CARGO_MANIFEST_DIR/src/root/templates"]
pub struct RootTemplates;

#[derive(RustEmbed)]
#[prefix = "server/"]
#[folder = "$CARGO_MANIFEST_DIR/src/server/templates"]
pub struct ServerTemplates;

#[derive(RustEmbed)]
#[prefix = "auth/"]
#[folder = "$CARGO_MANIFEST_DIR/src/auth/templates"]
pub struct AuthTemplates;

#[derive(RustEmbed)]
#[prefix = "users/"]
#[folder = "$CARGO_MANIFEST_DIR/src/users/templates"]
pub struct UsersTemplates;

#[derive(RustEmbed)]
#[prefix = "model/"]
#[folder = "$CARGO_MANIFEST_DIR/src/model/sql"]
pub struct ModelSqlTemplates;

#[derive(RustEmbed)]
#[prefix = "model/"]
#[folder = "$CARGO_MANIFEST_DIR/src/model/rust_templates/"]
pub struct ModelRustTemplates;

fn get_files<FILES: RustEmbed>() -> impl Iterator<Item = (String, Cow<'static, str>)> {
    FILES::iter().map(|f| {
        let filename = f.to_string();
        let data = FILES::get(&filename).unwrap();
        let data = match data.data {
            Cow::Borrowed(b) => Cow::Borrowed(std::str::from_utf8(b).unwrap()),
            Cow::Owned(s) => Cow::Owned(String::from_utf8(s).unwrap()),
        };
        (filename, data)
    })
}

fn create_tera() -> Tera {
    let mut tera = Tera::default();

    let template_files = get_files::<RootTemplates>()
        .chain(get_files::<AuthTemplates>())
        .chain(get_files::<ModelRustTemplates>())
        .chain(get_files::<ModelSqlTemplates>())
        .chain(get_files::<ServerTemplates>())
        .chain(get_files::<UsersTemplates>())
        .collect::<Vec<_>>();
    let res = tera.add_raw_templates(template_files);

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
            assert_eq!(call("don't".to_string()), "'don''t'");
        }

        #[test]
        fn array() {
            assert_eq!(
                sql_string_filter(&json!(["hello", "wo'rld"]), &HashMap::default()).unwrap(),
                json!(["'hello'", "'wo''rld'"])
            );
        }
    }
}
