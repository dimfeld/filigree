use clap::{Args, Parser, Subcommand};
use error_stack::{Report, ResultExt};
use filigree::{
    auth::{SameSiteArg, SessionCookieBuilder},
    tracing_config::{configure_tracing, teardown_tracing, TracingExportConfig},
};
use filigree_test_app::{db, server, util_cmd, Error};
use tracing::{event, Level};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Util(util_cmd::UtilCommand),
    Db(db::DbCommand),
    Serve(ServeCommand),
}

#[derive(Args, Debug)]
struct ServeCommand {
    /// The PostgreSQL database to connect to
    #[clap(long = "db", env = "DATABASE_URL")]
    database_url: String,

    /// The IP host to bind to
    #[clap(long, env = "HOST", default_value_t = String::from("127.0.0.1"))]
    host: String,

    /// The TCP port to listen on
    #[clap(long, env = "PORT", default_value_t = 7823)]
    port: u16,

    /// The environment in which this server is running
    #[clap(long = "env", env = "ENV", default_value_t = String::from("development"))]
    env: String,

    /// Request timeout, in seconds
    #[clap(long, env = "REQUEST_TIMEOUT", default_value_t = 60)]
    request_timeout: u64,

    #[clap(long, env = "COOKIE_SAME_SITE", value_enum, default_value_t = SameSiteArg::Strict)]
    cookie_same_site: SameSiteArg,

    #[clap(long, env = "COOKIE_INSECURE")]
    cookie_insecure: bool,

    /// Session expiry time, in days
    #[clap(long, env = "SESSION_EXPIRY", default_value_t = 7)]
    session_expiry: u64,

    /// Maintain at least this many connections to the database.
    #[clap(long, env = "DB_MIN_CONNECTIONS", default_value_t = 0)]
    db_min_connections: u32,

    /// Create no more than this many connections to the database.
    #[clap(long, env = "DB_MAX_CONNECTIONS", default_value_t = 100)]
    db_max_connections: u32,

    /// Require the user to verify their email before activating their account.
    #[clap(env = "REQUIRE_EMAIL_VERIFICATION", default_value_t = true)]
    require_email_verification: bool,

    /// The email service to use
    #[clap(env="EMAIL_SENDER_SERVICE", default_value_t = String::from("none"))]
    email_sender_service: String,

    /// The API token for the email sending service
    #[clap(env="EMAIL_SENDER_API_TOKEN", default_value_t = String::from(""))
    ]
    email_sender_api_token: String,

    /// The email address to use as the default sender
    #[clap(env="EMAIL_DEFAULT_FROM_ADDRESS", default_value_t = String::from("support@example.com"))]
    email_default_from_address: String,
    // tracing endpoint (if any)
    // honeycomb team
    // honeycomb dataset
    // jaeger service name
    // jaeger endpoint
}

async fn serve(cmd: ServeCommand) -> Result<(), Report<Error>> {
    error_stack::Report::set_color_mode(error_stack::fmt::ColorMode::None);

    // TODO make this configurable
    configure_tracing(
        "",
        TracingExportConfig::None,
        tracing_subscriber::fmt::time::ChronoUtc::rfc_3339(),
        std::io::stdout,
    )
    .change_context(Error::ServerStart)?;

    let pool_options = sqlx::postgres::PgPoolOptions::new()
        .min_connections(cmd.db_min_connections)
        .max_connections(cmd.db_max_connections);

    let pg_pool = if cmd.db_min_connections > 0 {
        pool_options.connect(&cmd.database_url).await
    } else {
        pool_options.connect_lazy(&cmd.database_url)
    };

    let pg_pool = pg_pool.change_context(Error::Db)?;

    db::run_migrations(&pg_pool).await?;

    let secure_cookies = !cmd.cookie_insecure;

    let email_service = filigree::email::services::email_service_from_name(
        &cmd.email_sender_service,
        cmd.email_sender_api_token,
    );
    let email_sender =
        filigree::email::services::EmailSender::new(cmd.email_default_from_address, email_service);

    let server = server::create_server(server::Config {
        env: cmd.env,
        host: cmd.host,
        port: cmd.port,
        request_timeout: std::time::Duration::from_secs(cmd.request_timeout),
        require_email_verification: cmd.require_email_verification,
        cookie_configuration: SessionCookieBuilder::new(secure_cookies, cmd.cookie_same_site),
        session_expiry: filigree::auth::ExpiryStyle::AfterIdle(std::time::Duration::from_secs(
            cmd.session_expiry * 24 * 60 * 60,
        )),
        email_sender,
        pg_pool,
    })
    .await?;

    server.run().await?;

    event!(Level::INFO, "Exporting remaining traces");
    teardown_tracing().await.change_context(Error::Shutdown)?;
    event!(Level::INFO, "Trace shut down complete");

    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
pub async fn main() -> Result<(), Report<Error>> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Command::Db(cmd) => cmd.handle().await?,
        Command::Serve(cmd) => serve(cmd).await?,
        Command::Util(cmd) => cmd.handle().await?,
    }

    Ok(())
}
