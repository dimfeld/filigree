use std::{borrow::Cow, collections::HashMap, error::Error, sync::OnceLock};

use tera::{Tera, Value};

static CELL: OnceLock<Tera> = OnceLock::new();

pub fn get_tera() -> &'static Tera {
    CELL.get_or_init(create_tera)
}

fn create_tera() -> Tera {
    let mut tera = Tera::default();

    let res = tera.add_raw_templates(vec![
        (
            "migrate_up.sql.tera",
            include_str!("model/sql/migrate_up.sql.tera"),
        ),
        (
            "migrate_down.sql.tera",
            include_str!("model/sql/migrate_down.sql.tera"),
        ),
        ("delete.sql.tera", include_str!("model/sql/delete.sql.tera")),
        ("insert.sql.tera", include_str!("model/sql/insert.sql.tera")),
        ("list.sql.tera", include_str!("model/sql/list.sql.tera")),
        (
            "select_one.sql.tera",
            include_str!("model/sql/select_one.sql.tera"),
        ),
        ("update.sql.tera", include_str!("model/sql/update.sql.tera")),
        ("sql_macros.tera", include_str!("model/sql/sql_macros.tera")),
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
