use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use glob::glob;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{
    format::Formatters,
    model::{Model, ModelAuthScope, SqlDialect},
    Error,
};

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "Config::default_port")]
    pub default_port: u16,

    #[serde(default)]
    pub formatter: Formatters,
    /// The SQL dialect to use. Defaults to postgresql
    #[serde(default = "Config::default_sql_dialect")]
    pub sql_dialect: SqlDialect,

    /// The auth scope for models that don't specify a different one.
    pub default_auth_scope: ModelAuthScope,
    // TODO implement this
    // /// Set to true to enable project-based object organization
    // #[serde(default)]
    // pub use_projects: bool,
    /// A prefix that will be used for all environment variable names when reading server
    /// configuration. Defaults to no prefix.
    /// e.g. setting env_prefix to "FOO_" will read the database URL from "FOO_DATABASE_URL"
    pub env_prefix: Option<String>,

    /// If set, the generated application will load .env files when it starts
    #[serde(default)]
    pub dotenv: bool,
}

impl Config {
    fn from_path(path: &Path) -> Result<Self, Report<Error>> {
        read_toml(path)
    }

    const fn default_sql_dialect() -> SqlDialect {
        SqlDialect::Postgresql
    }

    const fn default_port() -> u16 {
        7823
    }
}

#[derive(Deserialize)]
struct CargoToml {
    package: CargoTomlPackage,
}

#[derive(Deserialize)]
struct CargoTomlPackage {
    name: String,
}

#[derive(Debug)]
pub struct FullConfig {
    pub crate_name: String,
    pub config: Config,
    pub models: Vec<Model>,
}

impl FullConfig {
    pub fn from_dir(dir: Option<PathBuf>) -> Result<Self, Report<Error>> {
        let config_file_path = dir
            .map(|d| d.join("config.toml"))
            .or_else(|| {
                find_up_file(
                    &std::env::current_dir().expect("finding current directory"),
                    "filigree/config.toml",
                )
            })
            .ok_or(Error::ReadConfigFile)?;

        let dir = config_file_path.parent().ok_or(Error::ReadConfigFile)?;

        let config = Config::from_path(&config_file_path)?;

        let cargo_toml = dir
            .parent()
            .ok_or(Error::ReadConfigFile)?
            .join("Cargo.toml");
        let crate_name = read_toml::<CargoToml>(&cargo_toml)?.package.name;

        let models_glob = dir.join("models/*.toml");
        let models = glob(&models_glob.to_string_lossy())
            .expect("parsing glob")
            .map(|model_path| {
                let model_path = model_path.change_context(Error::ReadConfigFile)?;
                read_toml::<Model>(&model_path)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FullConfig {
            crate_name,
            config,
            models,
        })
    }
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T, Report<Error>> {
    let data = std::fs::read_to_string(path)
        .change_context(Error::Config)
        .attach_printable_lazy(|| path.display().to_string())?;
    let file: T = toml::from_str(&data).change_context(Error::Config)?;
    Ok(file)
}

pub fn find_up_file(start_path: &Path, target: &str) -> Option<PathBuf> {
    let start_path = start_path.canonicalize().ok()?;
    for p in start_path.ancestors() {
        let candidate = p.join(target);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}
