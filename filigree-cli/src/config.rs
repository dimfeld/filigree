pub mod custom_endpoint;
pub mod job;
pub mod storage;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use error_stack::{Report, ResultExt};
use glob::glob;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use self::{job::QueueConfig, storage::StorageConfig};
use crate::{
    format::FormatterConfig,
    model::{field::ModelField, Model, ModelAuthScope, SqlDialect},
    state::State,
    Error,
};

#[derive(Deserialize, Debug)]
pub struct Config {
    /// The name of the product/application
    pub product_name: String,
    /// The name of the company that makes the product, or your name if you prefer.
    pub company_name: String,

    /// The directory containing the Rust API, relative to the directory containing the `filigree`
    /// directory.
    /// Defaults to "."
    #[serde(default = "Config::default_api_dir")]
    pub api_dir: PathBuf,
    /// The directory containing the web UI, relative to the directory containing the `filigree`
    /// directory. Defaults to "./web"
    #[serde(default = "Config::default_web_dir")]
    pub web_dir: PathBuf,

    pub server: ServerConfig,

    /// A mapping of secret name to the environment variable used. If `env_prefix` is set, it will
    /// be prepended to the values here. The values here will become members of a `Secrets` struct
    /// in the `ServerState`.
    pub secrets: BTreeMap<String, String>,

    /// Full paths to types that exist in the Rust application and should be replicated in
    /// Typescript. These types must derive or otherwise implement the [schemars::JsonSchema] trait.
    #[serde(default)]
    pub shared_types: Vec<String>,

    #[serde(default)]
    pub formatter: FormatterConfig,
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
    /// Configuration for the database
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Configuration for sending emails
    pub email: EmailConfig,

    /// Configuration for user behavior
    #[serde(default)]
    pub users: UsersConfig,

    /// Configuration that extends built-in models
    #[serde(default)]
    pub extend: ExtendConfig,

    /// Storage locations and providers that the application uses
    #[serde(default)]
    pub storage: StorageConfig,

    /// Configuration for the job queue itself
    #[serde(default)]
    pub queue: QueueConfig,

    /// Configuration for background jobs
    #[serde(default)]
    pub job: BTreeMap<String, job::JobConfig>,

    /// Custom Worker configurations for background jobs. Currently all workers
    /// are in the same process as the API server.
    #[serde(default)]
    pub worker: BTreeMap<String, job::WorkerConfig>,

    #[serde(skip)]
    pub(crate) use_queue: bool,
}

impl Config {
    fn from_path(path: &Path) -> Result<Self, Report<Error>> {
        let mut config: Self = read_toml(path)?;
        config.use_queue = !config.job.is_empty() || !config.worker.is_empty();
        Ok(config)
    }

    pub fn default_api_dir() -> PathBuf {
        ".".into()
    }

    pub fn default_web_dir() -> PathBuf {
        "./web".into()
    }

    pub const fn default_sql_dialect() -> SqlDialect {
        SqlDialect::Postgresql
    }
}

#[derive(Serialize, Deserialize, serde_derive_default::Default, Debug)]
pub struct ServerConfig {
    /// If set, the generated application will load .env files when it starts
    #[serde(default)]
    pub dotenv: bool,

    /// The default port that the server should listen on
    #[serde(default = "default_port")]
    pub default_port: u16,

    /// The hosts that the server should assume are pointing to it.
    #[serde(default)]
    pub hosts: Vec<String>,

    /// A prefix that will be used for all environment variable names when reading server
    /// configuration. Defaults to no prefix.
    /// e.g. setting env_prefix to "FOO_" will read the database URL from "FOO_DATABASE_URL"
    pub env_prefix: Option<String>,

    /// How to configure CORS for the API routes
    #[serde(default)]
    pub api_cors: CorsSetting,

    /// The HTTP user agent to use when making requests from the API. If omitted, `product_name`
    /// from the main configuration will be used.
    pub user_agent: Option<String>,
}

const fn default_port() -> u16 {
    7823
}

/// Cross-origin Resource Sharing (CORS) configuration
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug)]
pub enum CorsSetting {
    /// Don't configure CORS at all, which prevents any cross-origin request from being accepted
    /// if nothing else in the request chain (e.g. a reverse proxy) sets the Access-Control headers.
    #[default]
    None,
    /// Allow cross-origin requests from any host in the `hosts` list
    AllowHostList,
    /// Allow all hosts to access /api routes. Cookies are still not permitted.
    AllowAll,
}

#[derive(Serialize, Deserialize, serde_derive_default::Default, Debug)]
pub struct DatabaseConfig {
    /// If true, migrations will be run automatically when starting the application
    #[serde(default)]
    pub migrate_on_start: bool,

    /// The minimum number of connections for the database pool to have open.
    /// This defaults to 0 which is appropriate when using a "serverless" Postgres
    /// provider that may charge you for resources incurred by holding open an idle
    /// connection, but you may want a higher value for better responsiveness
    /// when this is not a consideration.
    #[serde(default = "default_min_connections")]
    pub min_connections: u16,

    /// The maximum number of connections in the database pool.
    /// Defaults to 100
    #[serde(default = "default_max_connections")]
    pub max_connections: u16,
}

const fn default_min_connections() -> u16 {
    0
}

const fn default_max_connections() -> u16 {
    100
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

#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct UsersConfig {
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

/// Configuration that extends built-in data
#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct ExtendConfig {
    pub models: Option<ExtendModelsConfig>,
}

/// Extend the built-in user, role, and organization models
#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct ExtendModelsConfig {
    pub user: Option<ExtendModelConfig>,
    pub role: Option<ExtendModelConfig>,
    pub organization: Option<ExtendModelConfig>,
}

/// Extend a built-in model
#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct ExtendModelConfig {
    /// Add more fields to the model
    pub fields: Vec<ModelField>,
}

const fn true_t() -> bool {
    true
}

#[derive(Debug)]
pub struct FullConfig {
    pub crate_name: String,
    pub config: Config,
    pub models: Vec<Model>,
    pub state_dir: PathBuf,
    pub crate_manifest: cargo_toml::Manifest,
    pub state: State,
    pub api_dir: PathBuf,
    pub web_dir: PathBuf,
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

        let base_dir = dir.parent().ok_or(Error::ReadConfigFile)?.to_path_buf();
        let api_dir = base_dir.join(&config.api_dir);
        let web_dir = base_dir.join(&config.web_dir);

        let cargo_toml_path = api_dir.join("Cargo.toml");
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

        let state_dir = dir.join(".state");

        let mut state = State::from_dir(&state_dir);
        state.update_from_config(&config);
        state
            .save(&state_dir)
            .change_context(Error::WriteFile)
            .attach_printable("Saving state JSON")?;

        Ok(FullConfig {
            crate_name,
            config,
            models,
            state_dir,
            api_dir,
            web_dir,
            crate_manifest: manifest,
            state,
        })
    }
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T, Report<Error>> {
    let data = std::fs::read_to_string(path)
        .change_context(Error::Config)
        .attach_printable_lazy(|| path.display().to_string())?;
    let file: T = toml::from_str(&data)
        .change_context(Error::Config)
        .attach_printable_lazy(|| path.display().to_string())?;
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
