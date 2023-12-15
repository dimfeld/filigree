use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use glob::glob;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{
    format::Formatters,
    model::{Model, SqlDialect},
    Error,
};

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub formatter: Formatters,
    /// The SQL dialect to use. Defaults to postgresql
    #[serde(default = "Config::default_sql_dialect")]
    pub sql_dialect: SqlDialect,

    /// Where to place generated files.
    /// Defaults to src/generated
    #[serde(default = "Config::default_generated_path")]
    pub generated_path: PathBuf,

    /// Where the place the SQL migrations
    /// Defaults to `migrations`
    #[serde(default = "Config::default_migrations_path")]
    pub migrations_path: PathBuf,
}

impl Config {
    fn from_path(path: &Path) -> Result<Self, Report<Error>> {
        read_toml(path)
    }

    fn default_sql_dialect() -> SqlDialect {
        SqlDialect::Postgresql
    }

    fn default_generated_path() -> PathBuf {
        "src/generated".into()
    }

    fn default_migrations_path() -> PathBuf {
        "migrations".into()
    }
}

#[derive(Debug)]
pub struct FullConfig {
    pub config: Config,
    pub models: Vec<Model>,
}

impl FullConfig {
    pub fn from_dir(dir: &Path) -> Result<Self, Report<Error>> {
        let config = Config::from_path(&dir.join("config.toml"))?;

        let models_glob = dir.join("models/*.toml");
        let models = glob(&models_glob.to_string_lossy())
            .expect("parsing glob")
            .map(|model_path| {
                let model_path = model_path.change_context(Error::ReadConfigFile)?;
                read_toml::<Model>(&model_path)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FullConfig { config, models })
    }
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T, Report<Error>> {
    let data = std::fs::read_to_string(path)
        .change_context(Error::Config)
        .attach_printable_lazy(|| path.display().to_string())?;
    let file: T = toml::from_str(&data).change_context(Error::Config)?;
    Ok(file)
}
