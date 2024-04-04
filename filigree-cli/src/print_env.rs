use std::fmt::Display;

use clap::Args;
use convert_case::{Case, Casing};
use error_stack::Report;
use filigree::storage::StoragePreset;

use crate::{
    config::{
        job::WorkerConfig,
        storage::{StorageBucketConfig, StorageProviderConfig},
        FullConfig,
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

enum EnvPrintMode {
    /// Shell, dotenv
    Shell,
    Dockerfile,
    DockerRun,
}

struct PrintConfig {
    mode: EnvPrintMode,
    env_prefix: String,
    print_comments: bool,
}

fn print_var(config: &PrintConfig, key: &str, default_value: impl Display, desc: &str) {
    let comment = config.print_comments && !desc.is_empty();
    match config.mode {
        EnvPrintMode::Shell => {
            if comment {
                println!("# {desc}");
            }
            println!(
                "{env_prefix}{key}={default_value}",
                env_prefix = config.env_prefix
            );
        }
        EnvPrintMode::DockerRun => {
            if comment {
                println!("# {desc} \\");
            }
            println!(
                "--env {env_prefix}{key}='{default_value}' \\",
                env_prefix = config.env_prefix
            );
        }
        EnvPrintMode::Dockerfile => {
            if comment {
                println!("# {desc}");
            }
            println!(
                "ENV {env_prefix}{key}='{default_value}'",
                env_prefix = config.env_prefix
            );
        }
    }
}

pub fn run(config: FullConfig, args: Command) -> Result<(), Report<Error>> {
    let FullConfig { config, .. } = config;
    let pc = PrintConfig {
        mode: if args.dockerfile {
            EnvPrintMode::Dockerfile
        } else if args.docker_run {
            EnvPrintMode::DockerRun
        } else {
            EnvPrintMode::Shell
        },
        env_prefix: config.server.env_prefix.clone().unwrap_or_default(),
        print_comments: args.comments,
    };

    print_var(&pc, "DATABASE_URL", "", "Database URL to connect to");
    print_var(&pc, "HOST", "::1", "Host to bind to");
    print_var(&pc, "PORT", config.server.default_port, "Port to listen on");
    if config.server.forward_to_frontend {
        print_var(
            &pc,
            "FRONTEND_PORT",
            config.server.frontend_port,
            "Port to forward non-API frontend requests to",
        );
    }
    print_var(&pc, "ENV", "development", "");
    print_var(
        &pc,
        "SERVE_FRONTEND",
        "false",
        "Set to true to serve the frontend assets and forward non-API requests",
    );
    print_var(
        &pc,
        "FRONTEND_ASSET_DIR",
        "",
        "The directory where the frontend static assets are located",
    );
    print_var(
        &pc,
        "FRONTEND_PORT",
        config.server.frontend_port,
        "The port on which the frontend server is listening",
    );
    print_var(
        &pc,
        "REQUEST_TIMEOUT",
        60,
        "Request timeout for the default HTTP client used by the API",
    );
    print_var(
        &pc,
        "COOKIE_SAME_SITE",
        "Strict",
        "The SameSite setting to use when setting cookies",
    );

    print_var(
        &pc,
        "INSECURE",
        "false",
        "Set if the site is being accessed over unencrypted HTTP",
    );

    print_var(&pc, "SESSION_EXPIRY", 14, "Session expiry time in days");

    print_var(
        &pc,
        "DB_MIN_CONNECTIONS",
        config.database.min_connections,
        "The minimum number of database connections to keep open",
    );
    print_var(
        &pc,
        "DB_MAX_CONNECTIONS",
        config.database.max_connections,
        "The maximum number of database connections to open",
    );

    print_var(
        &pc,
        "EMAIL_SENDER_SERVICE",
        &config.email.provider,
        "The email sending service to use",
    );

    print_var(
        &pc,
        "EMAIL_SENDER_API_TOKEN",
        "",
        "The API token for the selected email sending service",
    );

    print_var(
        &pc,
        "EMAIL_DEFAULT_FROM_ADDRESS",
        &config.email.from,
        "The email address to use as the default sender",
    );

    print_var(
        &pc,
        "ALLOW_PUBLIC_SIGNUP",
        config.users.allow_public_signup,
        "Allow users to sign up themselves",
    );
    print_var(
        &pc,
        "ALLOW_INVITE_TO_SAME_ORG",
        config.users.allow_invite_to_same_org,
        "Allow users to invite people to their team",
    );
    print_var(
        &pc,
        "ALLOW_INVITE_TO_NEW_ORG",
        config.users.allow_invite_to_new_org,
        "Allow users to invite people to the app, in their own new team",
    );

    print_var(
        &pc,
        "SAME_ORG_INVITES_REQUIRE_EMAIL_VERIFICATION",
        &config.users.same_org_invites_require_email_verification,
        "Require email verification when inviting people to the same organization",
    );

    print_var(
        &pc,
        "HOSTS",
        "",
        "A list of hostnames that the server should recognize as belonging to it",
    );

    print_var(
        &pc,
        "API_CORS",
        config.server.api_cors,
        "The CORS configuration to use",
    );

    print_var(
        &pc,
        "OAUTH_REDIRECT_URL_BASE",
        "",
        "The base URL for OAuth redirect URLs. If omitted, `hosts[0]` is used.",
    );

    print_var(
        &pc,
        "OBFUSCATE_ERRORS",
        "",
        "Whether or not to obfuscate details from internal server errors. If omitted, the default is to obfuscate when env != \"development\".",
    );

    if config.use_queue {
        print_var(
            &pc,
            "QUEUE_PATH",
            "queue.db",
            "The filesystem location to store the task queue database",
        );
    }

    for (name, cfg) in &config.worker {
        print_worker_config_vars(&pc, name, cfg);
    }

    for (name, cfg) in &config.storage.provider {
        print_storage_provider_config_vars(&pc, name, cfg);
    }

    for (name, cfg) in &config.storage.bucket {
        print_storage_bucket_config_vars(&pc, name, cfg);
    }

    for (name, env) in config.secrets {
        print_var(&pc, env.as_str(), "", &format!("Value for secret '{name}'"))
    }

    Ok(())
}

fn print_worker_config_vars(pc: &PrintConfig, name: &str, cfg: &WorkerConfig) {
    let base = format!("WORKER_{name}_", name = name.to_case(Case::ScreamingSnake));

    print_var(
        &pc,
        &format!("{base}MIN_CONCURRENCY"),
        &cfg.min_concurrency(),
        "The worker will try to fetch more jobs when it is running fewer than this number.",
    );
    print_var(
        &pc,
        &format!("{base}MAX_CONCURRENCY"),
        &cfg.max_concurrency(),
        "The worker will run at most this many jobs.",
    );
}

fn print_storage_provider_config_vars(pc: &PrintConfig, name: &str, cfg: &StorageProviderConfig) {
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

    print_var(
        &pc,
        &format!("{base}PROVIDER_TYPE"),
        default_storage_type,
        "Which storage service type to use. One of s3 (for any s3-compatible service), local, or memory",
    );

    print_var(
        &pc,
        &format!("{base}LOCAL_BASE_PATH"),
        "",
        "When type is local, the base directory to use",
    );
    print_var(
        &pc,
        &format!("{base}S3_ENDPOINT"),
        name,
        "When type is s3, endpoint to use",
    );
    print_var(
        &pc,
        &format!("{base}S3_REGION"),
        "",
        "When type is s3, which region to ise",
    );
    print_var(
        &pc,
        &format!("{base}S3_ACCESS_KEY_ID"),
        "",
        "When type is s3, the access key id",
    );
    print_var(
        &pc,
        &format!("{base}S3_SECRET_ACCESS_KEY"),
        "",
        "When type is s3, the secret access key",
    );
    print_var(
        &pc,
        &format!("{base}S3_EXPRESS"),
        "false",
        "When type is S3, true if this is S3 Express",
    );
    print_var(
        &pc,
        &format!("{base}S3_VIRTUAL_HOST_STYLE"),
        "",
        "When type is S3, true to force virtual host style",
    );

    print_var(
        &pc,
        &format!("{base}R2_ACCOUNT_ID"),
        "",
        "When using the Cloudflare R2 preset, the account ID to use",
    );
}

fn print_storage_bucket_config_vars(pc: &PrintConfig, name: &str, cfg: &StorageBucketConfig) {
    let base = format!("STORAGE_{name}_", name = name.to_case(Case::ScreamingSnake));

    print_var(
        &pc,
        &format!("{base}BUCKET"),
        &cfg.bucket,
        "The name of the bucket to use",
    );
    print_var(
        &pc,
        &format!("{base}PUBLIC_URL"),
        &cfg.public_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default(),
        "The public URL at which this bucket is exposed, if any",
    );
}
