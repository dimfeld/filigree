use std::{fmt::Display, path::PathBuf};

use clap::Args;
use convert_case::{Case, Casing};
use error_stack::{Report, ResultExt};
use filigree::storage::StoragePreset;

use crate::{
    config::{
        job::WorkerConfig,
        storage::{StorageBucketConfig, StorageProviderConfig},
        Config, FullConfig,
    },
    Error,
};

#[derive(Args)]
pub struct Command {
    /// Print the environment variables in the format for a dockerfile
    #[clap(long)]
    dockerfile: bool,

    /// Print the environment variables in the format for a Docker run command
    #[clap(long)]
    docker_run: bool,

    /// Add comments for each environment variable
    #[clap(short, long)]
    comments: bool,
}

pub enum EnvPrintMode {
    /// Shell, dotenv
    Shell,
    Dockerfile,
    DockerRun,
}

pub struct PrintConfig {
    pub mode: EnvPrintMode,
    pub env_prefix: String,
    pub print_comments: bool,
}

fn print_var(
    writer: &mut dyn std::io::Write,
    config: &PrintConfig,
    key: &str,
    default_value: impl Display,
    desc: &str,
) -> Result<(), std::io::Error> {
    let comment = config.print_comments && !desc.is_empty();
    match config.mode {
        EnvPrintMode::Shell => {
            if comment {
                writeln!(writer, "# {desc}")?;
            }
            writeln!(
                writer,
                "{env_prefix}{key}={default_value}",
                env_prefix = config.env_prefix
            )?;
        }
        EnvPrintMode::DockerRun => {
            if comment {
                writeln!(writer, "# {desc} \\")?;
            }
            writeln!(
                writer,
                "--env {env_prefix}{key}='{default_value}' \\",
                env_prefix = config.env_prefix
            )?;
        }
        EnvPrintMode::Dockerfile => {
            if comment {
                writeln!(writer, "# {desc}")?;
            }
            writeln!(
                writer,
                "ENV {env_prefix}{key}='{default_value}'",
                env_prefix = config.env_prefix
            )?;
        }
    }

    Ok(())
}

pub fn run(config: FullConfig, args: Command) -> Result<(), Report<Error>> {
    let pc = PrintConfig {
        mode: if args.dockerfile {
            EnvPrintMode::Dockerfile
        } else if args.docker_run {
            EnvPrintMode::DockerRun
        } else {
            EnvPrintMode::Shell
        },
        env_prefix: config.config.server.env_prefix.clone().unwrap_or_default(),
        print_comments: args.comments,
    };

    let web_relative_to_api = config.web_relative_to_api();

    write_env_vars(
        &mut std::io::stdout(),
        config.config,
        web_relative_to_api,
        EnvVarOverrides::default(),
        &pc,
    )
    .change_context(Error::WriteFile)?;
    Ok(())
}

#[derive(Default)]
pub struct EnvVarOverrides {
    pub database_url: Option<String>,
    pub dev: Option<bool>,
}

pub fn write_env_vars(
    writer: &mut dyn std::io::Write,
    config: Config,
    web_relative_to_api: PathBuf,
    overrides: EnvVarOverrides,
    pc: &PrintConfig,
) -> Result<(), std::io::Error> {
    print_var(
        writer,
        &pc,
        "READ_DOTENV",
        config.server.dotenv,
        "Read .env file when starting",
    )?;
    print_var(
        writer,
        &pc,
        "DATABASE_URL",
        overrides.database_url.unwrap_or_default(),
        "Database URL to connect to",
    )?;
    print_var(writer, &pc, "HOST", "::1", "Host to bind to")?;
    print_var(
        writer,
        &pc,
        "PORT",
        config.server.default_port,
        "Port to listen on",
    )?;
    print_var(writer, &pc, "ENV", "development", "")?;
    print_var(writer, &pc, "LOG", "info", "Trace logging level to use")?;

    match config.error_reporting.provider {
        crate::config::ErrorReportingProvider::Sentry => {
            print_var(
                writer,
                &pc,
                "SENTRY_DSN",
                "",
                "Sentry DSN to use for error reporting",
            )?;
        }
        _ => {}
    }

    print_var(
        writer,
        &pc,
        "TRACING_TYPE",
        config.tracing.provider,
        "Tracing provider to use",
    )?;
    print_var(
        writer,
        &pc,
        "OTEL_SERVICE_NAME",
        &config.tracing.api_service_name,
        "The service name for the API service. If omitted, `api` is used.",
    )?;

    print_var(writer,
        &pc,
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        config.tracing.endpoint.as_deref().unwrap_or_default(),
        "The endpoint to send traces to. This can be omitted for Honeycomb but is required to be specified here or in the environment for other OTLP collectors.",
    )?;
    print_var(
        writer,
        &pc,
        "HONEYCOMB_API_KEY",
        "",
        "Honeycomb API key. Required when using Honeycomb tracing",
    )?;

    let default_web_asset_dir = config.web.files(&web_relative_to_api);
    print_var(
        writer,
        &pc,
        "WEB_ASSET_DIR",
        default_web_asset_dir.clone().unwrap_or_default(),
        "The directory where the frontend static assets are located",
    )?;

    if config.web.has_api_pages() {
        let default_manifest = default_web_asset_dir
            .as_ref()
            .map(|s| format!("{s}/.vite/manifest.json"))
            .unwrap_or_default();
        print_var(
            writer,
            &pc,
            "VITE_MANIFEST",
            default_manifest,
            "The location of the Vite manifest",
        )?;
        print_var(
            writer,
            &pc,
            "DEV",
            overrides.dev.unwrap_or(false),
            "Watch the filesystem for changes and enable live reload",
        )?;
    } else {
        print_var(
            writer,
            &pc,
            "WEB_PORT",
            config.web.port().map(|p| p.to_string()).unwrap_or_default(),
            "Port to forward non-API frontend requests to",
        )?;
    }

    print_var(
        writer,
        &pc,
        "REQUEST_TIMEOUT",
        60,
        "Request timeout for the default HTTP client used by the API",
    )?;
    print_var(
        writer,
        &pc,
        "COOKIE_SAME_SITE",
        "Strict",
        "The SameSite setting to use when setting cookies",
    )?;

    print_var(
        writer,
        &pc,
        "INSECURE",
        "false",
        "Set if the site is being accessed over unencrypted HTTP",
    )?;

    print_var(
        writer,
        &pc,
        "SESSION_EXPIRY",
        14,
        "Session expiry time in days",
    )?;

    print_var(
        writer,
        &pc,
        "DB_MIN_CONNECTIONS",
        config.database.min_connections,
        "The minimum number of database connections to keep open",
    )?;
    print_var(
        writer,
        &pc,
        "DB_MAX_CONNECTIONS",
        config.database.max_connections,
        "The maximum number of database connections to open",
    )?;

    print_var(
        writer,
        &pc,
        "EMAIL_SENDER_SERVICE",
        &config.email.provider,
        "The email sending service to use",
    )?;

    print_var(
        writer,
        &pc,
        "EMAIL_SENDER_API_TOKEN",
        "",
        "The API token for the selected email sending service",
    )?;

    print_var(
        writer,
        &pc,
        "EMAIL_DEFAULT_FROM_ADDRESS",
        &config.email.from,
        "The email address to use as the default sender",
    )?;

    print_var(
        writer,
        &pc,
        "ALLOW_PUBLIC_SIGNUP",
        config.users.allow_public_signup,
        "Allow users to sign up themselves",
    )?;
    print_var(
        writer,
        &pc,
        "ALLOW_INVITE_TO_SAME_ORG",
        config.users.allow_invite_to_same_org,
        "Allow users to invite people to their team",
    )?;
    print_var(
        writer,
        &pc,
        "ALLOW_INVITE_TO_NEW_ORG",
        config.users.allow_invite_to_new_org,
        "Allow users to invite people to the app, in their own new team",
    )?;

    print_var(
        writer,
        &pc,
        "SAME_ORG_INVITES_REQUIRE_EMAIL_VERIFICATION",
        &config.users.same_org_invites_require_email_verification,
        "Require email verification when inviting people to the same organization",
    )?;

    print_var(
        writer,
        &pc,
        "HOSTS",
        "",
        "A list of hostnames that the server should recognize as belonging to it",
    )?;

    print_var(
        writer,
        &pc,
        "API_CORS",
        config.server.api_cors,
        "The CORS configuration to use",
    )?;

    print_var(
        writer,
        &pc,
        "OAUTH_REDIRECT_URL_BASE",
        "",
        "The base URL for OAuth redirect URLs. If omitted, `hosts[0]` is used.",
    )?;

    print_var(writer,
        &pc,
        "OBFUSCATE_ERRORS",
        "",
        "Whether or not to obfuscate details from internal server errors. If omitted, the default is to obfuscate when env != \"development\".",
    )?;

    if config.use_queue {
        print_var(
            writer,
            &pc,
            "QUEUE_PATH",
            "queue.db",
            "The filesystem location to store the task queue database",
        )?;
    }

    for (name, cfg) in &config.worker {
        print_worker_config_vars(writer, &pc, name, cfg)?;
    }

    for (name, cfg) in &config.storage.provider {
        print_storage_provider_config_vars(writer, &pc, name, cfg)?;
    }

    for (name, cfg) in &config.storage.bucket {
        print_storage_bucket_config_vars(writer, &pc, name, cfg)?;
    }

    for (name, env) in config.secrets {
        print_var(
            writer,
            &pc,
            env.as_str(),
            "",
            &format!("Value for secret '{name}'"),
        )?;
    }

    Ok(())
}

fn print_worker_config_vars(
    writer: &mut dyn std::io::Write,
    pc: &PrintConfig,
    name: &str,
    cfg: &WorkerConfig,
) -> Result<(), std::io::Error> {
    let base = format!("WORKER_{name}_", name = name.to_case(Case::ScreamingSnake));

    print_var(
        writer,
        &pc,
        &format!("{base}MIN_CONCURRENCY"),
        &cfg.min_concurrency(),
        "The worker will try to fetch more jobs when it is running fewer than this number.",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}MAX_CONCURRENCY"),
        &cfg.max_concurrency(),
        "The worker will run at most this many jobs.",
    )?;

    Ok(())
}

fn print_storage_provider_config_vars(
    writer: &mut dyn std::io::Write,
    pc: &PrintConfig,
    name: &str,
    cfg: &StorageProviderConfig,
) -> Result<(), std::io::Error> {
    let base = format!(
        "STORAGE_PROVIDER_{name}_",
        name = name.to_case(Case::ScreamingSnake)
    );

    let default_storage_type = match cfg {
        StorageProviderConfig::Preconfigured(StoragePreset::S3 { .. }) => "s3",
        StorageProviderConfig::Preconfigured(StoragePreset::DigitalOceanSpaces { .. }) => "s3",
        StorageProviderConfig::Preconfigured(StoragePreset::BackblazeB2 { .. }) => "s3",
        StorageProviderConfig::Preconfigured(StoragePreset::CloudflareR2 { .. }) => "s3",
        StorageProviderConfig::Custom(filigree::storage::StorageConfig::S3(_)) => "s3",
        StorageProviderConfig::Custom(filigree::storage::StorageConfig::Local(_)) => "local",
        StorageProviderConfig::Custom(filigree::storage::StorageConfig::Memory) => "memory",
    };

    print_var(writer,
        &pc,
        &format!("{base}PROVIDER_TYPE"),
        default_storage_type,
        "Which storage service type to use. One of s3 (for any s3-compatible service), local, or memory",
    )?;

    print_var(
        writer,
        &pc,
        &format!("{base}LOCAL_BASE_PATH"),
        "",
        "When type is local, the base directory to use",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}S3_ENDPOINT"),
        name,
        "When type is s3, endpoint to use",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}S3_REGION"),
        "",
        "When type is s3, which region to ise",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}S3_ACCESS_KEY_ID"),
        "",
        "When type is s3, the access key id",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}S3_SECRET_ACCESS_KEY"),
        "",
        "When type is s3, the secret access key",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}S3_EXPRESS"),
        "false",
        "When type is S3, true if this is S3 Express",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}S3_VIRTUAL_HOST_STYLE"),
        "",
        "When type is S3, true to force virtual host style",
    )?;

    print_var(
        writer,
        &pc,
        &format!("{base}R2_ACCOUNT_ID"),
        "",
        "When using the Cloudflare R2 preset, the account ID to use",
    )?;

    Ok(())
}

fn print_storage_bucket_config_vars(
    writer: &mut dyn std::io::Write,
    pc: &PrintConfig,
    name: &str,
    cfg: &StorageBucketConfig,
) -> Result<(), std::io::Error> {
    let base = format!("STORAGE_{name}_", name = name.to_case(Case::ScreamingSnake));

    print_var(
        writer,
        &pc,
        &format!("{base}BUCKET"),
        &cfg.bucket,
        "The name of the bucket to use",
    )?;
    print_var(
        writer,
        &pc,
        &format!("{base}PUBLIC_URL"),
        &cfg.public_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default(),
        "The public URL at which this bucket is exposed, if any",
    )?;

    Ok(())
}
