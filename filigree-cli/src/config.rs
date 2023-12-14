use std::path::PathBuf;

use serde::Deserialize;

use crate::model::{Model, SqlDialect};

pub struct State {
    base_dir: PathBuf,
    config: Config,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    formatter: Formatters,
    sql_dialect: SqlDialect,
    models: Vec<Model>,
}

#[derive(Deserialize, Debug)]
pub struct Formatters {
    /// The formatter to use for Rust code. Defaults to rustfmt.
    pub rust: Option<String>,
    /// The formatter to use for SQL files.
    pub sql: Option<String>,
}
