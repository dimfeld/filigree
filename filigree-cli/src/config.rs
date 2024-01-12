use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};
use glob::glob;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    format::Formatters,
    model::{Model, ModelAuthScope, SqlDialect},
    Error,
};

#[derive(Deserialize, Debug)]
pub struct Config {
    /// The name of the product/application
    pub product_name: String,
    /// The name of the company that makes the product, or your name if you prefer.
    pub company_name: String,

    #[serde(default = "Config::default_port")]
    pub default_port: u16,

    #[serde(default)]
    pub formatter: Formatters,
    // Maybe support SQLite some day
    // /// The SQL dialect to use. Defaults to postgresql
    // #[serde(default = "Config::default_sql_dialect")]
    // pub sql_dialect: SqlDialect,
    /// TODO put this in a models config, along with the create permission setting
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

    /// Configuration for the database
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Configuration for sending emails
    pub email: EmailConfig,

    /// Configuration for user behavior
    #[serde(default)]
    pub users: UsersConfig,
}

impl Config {
    fn from_path(path: &Path) -> Result<Self, Report<Error>> {
        read_toml(path)
    }

    pub const fn default_sql_dialect() -> SqlDialect {
        SqlDialect::Postgresql
    }

    const fn default_port() -> u16 {
        7823
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DatabaseConfig {
    /// If true, migrations will be run automatically when starting the application
    #[serde(default)]
    pub migrate_on_start: bool,

    /// The minimum number of connections for the database pool to have open.
    /// This defaults to 0 which is appropriate when using a "serverless" Postgres
    /// provider that may charge you for resources incurred by holding open an idle
    /// connection, but you may want a higher value for better responsiveness
    /// when this is not a consideration.
    #[serde(default = "DatabaseConfig::default_min_connections")]
    pub min_connections: u16,

    /// The maximum number of connections in the database pool.
    /// Defaults to 100
    #[serde(default = "DatabaseConfig::default_max_connections")]
    pub max_connections: u16,
}

impl DatabaseConfig {
    const fn default_min_connections() -> u16 {
        0
    }

    const fn default_max_connections() -> u16 {
        100
    }
}

/// Configuration for email-related settings
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailConfig {
    /// The email service to use
    provider: EmailProvider,

    /// The address that emails are sent from, if not otherwise specified.
    from: String,
}

/// A choice of email service
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailProvider {
    /// No email service. This can be useful when first starting out a project.
    None,
    /// Send emails using Resend
    Resend,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UsersConfig {
    /// If true, require that users verify their email address after registering.
    /// Defaults to true.
    #[serde(default = "true_t")]
    pub require_email_verification: bool,

    /// Configure who is able to create new user accounts. Defaults to public signup
    /// Defaults to true.
    #[serde(default = "true_t")]
    pub allow_public_signup: bool,

    /// Allow inviting users to the same organization
    /// Defaults to true.
    #[serde(default = "true_t")]
    pub allow_invite_to_same_org: bool,

    /// When inviting new users to join your organization, require them to verify their email first.
    /// Defaults to true.
    #[serde(default = "true_t")]
    pub same_org_invites_require_email_verification: bool,

    /// Allow inviting users to be placed in a new organization
    /// Defaults to true.
    #[serde(default = "true_t")]
    pub allow_invite_to_new_org: bool,
}

const fn true_t() -> bool {
    true
}

#[derive(Debug)]
pub struct FullConfig {
    pub crate_name: String,
    pub config: Config,
    pub models: Vec<Model>,
    pub crate_manifest: cargo_toml::Manifest,
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

        let cargo_toml_path = dir
            .parent()
            .ok_or(Error::ReadConfigFile)?
            .join("Cargo.toml");
        let manifest = cargo_toml::Manifest::from_path(&cargo_toml_path)
            .change_context(Error::ReadConfigFile)
            .attach_printable_lazy(|| cargo_toml_path.display().to_string())?;
        let crate_name = manifest
            .package
            .as_ref()
            .ok_or(Error::ReadConfigFile)
            .attach_printable("Cargo.toml has no crate name")?
            .name
            .clone();

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
            crate_manifest: manifest,
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
