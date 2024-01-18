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
    auth::{CorsSetting, ExpiryStyle, SessionBackend, SessionCookieBuilder},
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

use crate::error::Error;

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

#[derive(Clone)]
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
    pub listener: tokio::net::TcpListener,
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
        axum::serve(self.listener, self.app)
            .with_graceful_shutdown(shutdown)
            .await
            .change_context(Error::ServerStart)?;
        event!(Level::INFO, "Shutting down server");

        // Can do extra shutdown tasks here.

        Ok(())
    }
}

/// Configuration for the server
pub struct Config {
    /// The environment we're running in. Currently this just distinguishes between
    /// "development" and any other value.
    pub env: String,
    /// The host to bind to.
    pub host: String,
    /// The port to bind to
    pub port: u16,
    /// True if the site is being hosted on plain HTTP. This should only be set in a development
    /// or testing environment.
    pub insecure: bool,
    /// How long to wait before timing out a request
    pub request_timeout: std::time::Duration,
    pub pg_pool: PgPool,

    pub cookie_configuration: SessionCookieBuilder,
    pub session_expiry: ExpiryStyle,
    pub new_user_flags: filigree::server::NewUserFlags,
    pub email_sender: filigree::email::services::EmailSender,

    pub hosts: Vec<String>,
    pub api_cors: filigree::auth::CorsSetting,
}

/// Create the server and return it, ready to run.
pub async fn create_server(config: Config) -> Result<Server, Report<Error>> {
    let production = config.env != "development" && !cfg!(debug_assertions);

    let host_values = config
        .hosts
        .iter()
        .map(|h| h.parse::<http::header::HeaderValue>())
        .collect::<Result<Vec<_>, _>>()
        .change_context(Error::ServerStart)
        .attach_printable("Unable to parse hosts list")?;

    let state = ServerState(Arc::new(ServerStateInner {
        production,
        filigree: Arc::new(FiligreeState {
            db: config.pg_pool.clone(),
            email: config.email_sender,
            new_user_flags: config.new_user_flags,
            hosts: config.hosts,
            session_backend: SessionBackend::new(
                config.pg_pool.clone(),
                config.cookie_configuration,
                config.session_expiry,
            ),
        }),
        insecure: config.insecure,
        db: config.pg_pool.clone(),
    }));

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
        .merge(crate::models::create_routes())
        .merge(crate::users::users::create_routes())
        .merge(crate::auth::create_routes())
        .layer(
            ServiceBuilder::new()
                .layer(panic_handler(production))
                .layer(ObfuscateErrorLayer::new(ObfuscateErrorLayerSettings {
                    enabled: production,
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

    let bind_ip = config
        .host
        .parse::<IpAddr>()
        .change_context(Error::ServerStart)?;
    let bind_addr = SocketAddr::from((bind_ip, config.port));
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .change_context(Error::ServerStart)?;

    let actual_addr = listener.local_addr().change_context(Error::ServerStart)?;
    let port = actual_addr.port();
    event!(Level::INFO, "Listening on {}:{port}", config.host);

    Ok(Server {
        host: config.host,
        port,
        app,
        state,
        listener,
    })
}
