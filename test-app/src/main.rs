use clap::{Args, Parser, Subcommand};
use error_stack::{Report, ResultExt};
use filigree::tracing_config::{configure_tracing, teardown_tracing, TracingExportConfig};
use filigree_test_app::{server, Error};
use tracing::{event, Level};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    // TODO bootstrap DB command
    // TODO migrate command
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
    // tracing endpoint (if any)
    // honeycomb team
    // honeycomb dataset
    // jaeger service name
    // jaeger endpoint
}

async fn serve(cmd: ServeCommand) -> Result<(), Report<Error>> {
    // TODO make this configurable
    configure_tracing("", TracingExportConfig::None).change_context(Error::ServerStart)?;

    let pg_pool = sqlx::PgPool::connect(&cmd.database_url)
        .await
        .change_context(Error::Db)?;

    let server = server::create_server(server::Config {
        env: cmd.env,
        host: cmd.host,
        port: cmd.port,
        request_timeout: std::time::Duration::from_secs(cmd.request_timeout),
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
    let cli = Cli::parse();

    match cli.command {
        Command::Serve(cmd) => serve(cmd).await,
    }
}
