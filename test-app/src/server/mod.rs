// Dependencies
// axum = "0.7.2"
// axum-auth = "0.4.1"
// axum-extra = { version = "0.9.0", features = ["typed-routing", "form"] }
// error-stack = { version = "0.4.1", features = ["eyre"] }
// hyper = "^0.14"
// tower = "0.4.13"
// tower-http = { version = "0.4.4", features = ["util", "catch-panic", "request-id", "trace", "limit", "compression-deflate", "compression-gzip", "compression-zstd", "decompression-full"] }
// tracing = "0.1.40"

use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::{extract::FromRef, routing::get, Router};
use error_stack::{Report, ResultExt};
use filigree::{
    auth::{
        oauth::providers::OAuthProvider, CorsSetting, ExpiryStyle, SessionBackend,
        SessionCookieBuilder,
    },
    errors::{panic_handler, ObfuscateErrorLayer, ObfuscateErrorLayerSettings},
    server::FiligreeState,
};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::MakeRequestUuid,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::{event, Level};

use crate::{error::Error, storage};

mod health;
mod meta;
#[cfg(test)]
mod tests;

/// Shared state used by the server
pub struct ServerStateInner {
    /// If the app is running in production mode. This should be used sparingly as there should be
    /// a minimum of difference between production and development to prevent bugs.
    pub production: bool,
    /// If the app is being hosted on plain HTTP
    pub insecure: bool,
    /// State for internal filigree endpoints
    pub filigree: Arc<FiligreeState>,
    /// The Postgres database connection pool
    pub db: PgPool,
    pub queue: effectum::Queue,
    /// Object storage providers
    pub storage: storage::AppStorage,
}

impl ServerStateInner {
    pub fn site_scheme(&self) -> &'static str {
        if self.insecure {
            "http"
        } else {
            "https"
        }
    }
}

impl std::ops::Deref for ServerStateInner {
    type Target = FiligreeState;

    fn deref(&self) -> &Self::Target {
        &self.filigree
    }
}

impl std::fmt::Debug for ServerStateInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerStateInner")
            .field("production", &self.production)
            .field("insecure", &self.insecure)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ServerState(Arc<ServerStateInner>);

impl std::ops::Deref for ServerState {
    type Target = ServerStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRef<ServerState> for Arc<FiligreeState> {
    fn from_ref(inner: &ServerState) -> Self {
        inner.0.filigree.clone()
    }
}

impl FromRef<ServerState> for PgPool {
    fn from_ref(inner: &ServerState) -> Self {
        inner.0.db.clone()
    }
}

impl FromRef<ServerState> for SessionBackend {
    fn from_ref(inner: &ServerState) -> Self {
        inner.0.session_backend.clone()
    }
}

/// The server and related information
pub struct Server {
    /// The host the server is bound to
    pub host: String,
    /// The port the server is bound to
    pub port: u16,
    /// The server itself.
    pub app: Router<()>,
    /// The server state.
    pub state: ServerState,
    /// The server's TCP listener
    pub listener: tokio::net::TcpListener,
    pub queue_workers: crate::jobs::QueueWorkers,
}

impl Server {
    /// Run the server, and perform a graceful shutdown when receiving a ctrl+c (SIGINT or
    /// equivalent).
    pub async fn run(self) -> Result<(), Report<Error>> {
        let shutdown = filigree::server::shutdown_signal();
        self.run_with_shutdown_signal(shutdown).await
    }

    /// Run the server, and shut it down when `shutdown_rx` closes.
    pub async fn run_with_shutdown_signal(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), Report<Error>> {
        event!(Level::INFO, "Shutting down server");
        axum::serve(self.listener, self.app)
            .with_graceful_shutdown(shutdown)
            .await
            .change_context(Error::ServerStart)?;

        event!(Level::INFO, "Shutting down queue");
        self.queue_workers.shutdown().await;
        // Once the workers are shut down, there's nothing else to do but close the queue for good
        // measure.
        self.state
            .queue
            .close(std::time::Duration::from_secs(30))
            .await
            .ok();

        // Can do extra shutdown tasks here.

        Ok(())
    }
}

/// Create a TCP listener.
pub async fn create_tcp_listener(
    host: &str,
    port: u16,
) -> Result<tokio::net::TcpListener, Report<Error>> {
    let bind_ip = host.parse::<IpAddr>().change_context(Error::ServerStart)?;
    let bind_addr = SocketAddr::from((bind_ip, port));
    tokio::net::TcpListener::bind(bind_addr)
        .await
        .change_context(Error::ServerStart)
}

pub enum ServerBind {
    /// A host and port to bind to
    HostPort(String, u16),
    /// An existing TCP listener to use
    Listener(tokio::net::TcpListener),
}

/// Configuration for the server
pub struct Config {
    /// The environment we're running in. Currently this just distinguishes between
    /// "development" and any other value.
    pub env: String,
    /// The host and port to bind to, or an existing TCP listener
    pub bind: ServerBind,
    /// True if the site is being hosted on plain HTTP. This should only be set in a development
    /// or testing environment.
    pub insecure: bool,
    /// How long to wait before timing out a request
    pub request_timeout: std::time::Duration,
    pub pg_pool: PgPool,

    pub cookie_configuration: SessionCookieBuilder,
    /// When user sessions should expire.
    pub session_expiry: ExpiryStyle,
    /// Flags controlling how new users are able to sign up or be invited.
    pub new_user_flags: filigree::server::NewUserFlags,
    /// The email sending service to use.
    pub email_sender: filigree::email::services::EmailSender,

    /// Whether or not to obfuscate details from internal server errors. If omitted,
    /// the default is to obfuscate when env != "development".
    pub obfuscate_errors: Option<bool>,

    pub hosts: Vec<String>,
    pub api_cors: filigree::auth::CorsSetting,

    /// The base URL for OAuth redirect URLs.
    pub oauth_redirect_url_base: String,
    /// Set the OAuth providers. If this is None, OAuth providers will be configured based on the
    /// environment variables present for each provider. See
    /// [filigree::auth::oauth::providers::create_supported_providers] for the logic there.
    ///
    /// OAuth can be disabled, regardless of environment variable settings, but passing `Some(Vec::new())`.
    pub oauth_providers: Option<Vec<Box<dyn OAuthProvider>>>,

    /// The path to the queue file
    pub queue_path: std::path::PathBuf,
    /// Set up recurring jobs. This should generally be true for normal operation and false for
    /// testing.
    pub init_recurring_jobs: bool,

    pub storage: storage::AppStorageConfig,
}

/// Create the server and return it, ready to run.
pub async fn create_server(config: Config) -> Result<Server, Report<Error>> {
    let production = config.env != "development" && !cfg!(debug_assertions);
    let obfuscate_errors = config.obfuscate_errors.unwrap_or(production);

    let host_values = config
        .hosts
        .iter()
        .map(|h| h.parse::<http::header::HeaderValue>())
        .collect::<Result<Vec<_>, _>>()
        .change_context(Error::ServerStart)
        .attach_printable("Unable to parse hosts list")?;

    let oauth_redirect_base = format!("{}/auth/oauth/login", config.oauth_redirect_url_base);
    let http_client = reqwest::Client::builder()
        .user_agent("Filigree Test App")
        .build()
        .unwrap();

    let queue = crate::jobs::create_queue(&config.queue_path)
        .await
        .change_context(Error::ServerStart)?;

    let state = ServerState(Arc::new(ServerStateInner {
        production,
        filigree: Arc::new(FiligreeState {
            http_client,
            db: config.pg_pool.clone(),
            email: config.email_sender,
            new_user_flags: config.new_user_flags,
            hosts: config.hosts,
            user_creator: Box::new(crate::users::users::UserCreator),
            oauth_providers: config.oauth_providers.unwrap_or_else(|| {
                filigree::auth::oauth::providers::create_supported_providers(
                    "",
                    &oauth_redirect_base,
                )
            }),
            session_backend: SessionBackend::new(
                config.pg_pool.clone(),
                config.cookie_configuration,
                config.session_expiry,
            ),
        }),
        insecure: config.insecure,
        db: config.pg_pool.clone(),
        queue,

        storage: storage::AppStorage::new(config.storage).change_context(Error::ServerStart)?,
    }));

    let queue_workers = crate::jobs::init(&state, config.init_recurring_jobs)
        .await
        .change_context(Error::ServerStart)?;

    let auth_queries = Arc::new(crate::auth::AuthQueries::new(
        config.pg_pool,
        config.session_expiry,
    ));

    let api_cors_layer = match config.api_cors {
        CorsSetting::None => CorsLayer::new(),
        CorsSetting::AllowAll => CorsLayer::permissive().max_age(Duration::from_secs(60 * 60)),
        CorsSetting::AllowHostList => CorsLayer::new()
            .allow_origin(host_values)
            .allow_methods(tower_http::cors::Any)
            .max_age(Duration::from_secs(60 * 60)),
    };

    let api_routes: Router<ServerState> = Router::new()
        .route("/healthz", get(health::healthz))
        .nest("/meta", meta::create_routes())
        .merge(filigree::auth::endpoints::create_routes())
        .merge(filigree::auth::oauth::create_routes())
        .merge(crate::models::create_routes())
        .merge(crate::users::users::create_routes())
        .merge(crate::auth::create_routes())
        .layer(
            ServiceBuilder::new()
                .layer(panic_handler(production))
                .layer(ObfuscateErrorLayer::new(ObfuscateErrorLayerSettings {
                    enabled: obfuscate_errors,
                    ..Default::default()
                }))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO))
                        .on_request(DefaultOnRequest::new().level(Level::INFO))
                        .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
                )
                .layer(TimeoutLayer::new(config.request_timeout))
                .layer(api_cors_layer)
                .layer(CompressionLayer::new())
                .layer(tower_cookies::CookieManagerLayer::new())
                .set_x_request_id(MakeRequestUuid)
                .propagate_x_request_id()
                .decompression()
                .layer(filigree::auth::middleware::AuthLayer::new(auth_queries))
                .into_inner(),
        );

    let api_routes: Router<()> = api_routes.with_state(state.clone());

    let app = Router::new().nest("/api", api_routes);

    let listener = match config.bind {
        ServerBind::Listener(l) => l,
        ServerBind::HostPort(host, port) => create_tcp_listener(&host, port).await?,
    };

    let actual_addr = listener.local_addr().change_context(Error::ServerStart)?;
    let port = actual_addr.port();
    let host = actual_addr.ip().to_string();
    event!(Level::INFO, "Listening on {host}:{port}");

    Ok(Server {
        host,
        port,
        app,
        state,
        listener,
        queue_workers,
    })
}
