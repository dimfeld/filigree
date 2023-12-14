use std::path::PathBuf;

use serde::Deserialize;

use crate::model::{Model, SqlDialect};

pub struct State {
    base_dir: PathBuf,
    config: Config,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    formatter: Option<Formatters>,
    /// The SQL dialect to use. Defaults to postgresql
    #[serde(default = "default_sql_dialect")]
    sql_dialect: SqlDialect,
    models: Vec<Model>,
    /// Where to place generated files.
    /// Defaults to src/generated
    #[serde(default = "default_generated_path")]
    generated_path: PathBuf,
}

fn default_sql_dialect() -> SqlDialect {
    SqlDialect::Postgresql
}

fn default_generated_path() -> PathBuf {
    "src/generated".into()
}

#[derive(Deserialize, Debug)]
pub struct Formatters {
    /// The formatter to use for Rust code. Defaults to rustfmt.
    pub rust: Option<String>,
    /// The formatter to use for SQL files.
    pub sql: Option<String>,
}
