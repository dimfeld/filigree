use axum::extract::Host;
use futures::FutureExt;
use tokio::signal;
use tracing::{event, Level};

#[cfg(feature = "local_auth")]
use crate::{
    auth::{oauth::providers::OAuthProvider, SessionBackend},
    users::users::UserCreator,
};
use crate::{email::services::EmailSender, error_reporting::ErrorReporter};

/// Internal state used by the server
pub struct FiligreeState {
    /// The database connection pool
    pub db: sqlx::PgPool,

    #[cfg(feature = "local_auth")]
    /// User session backend
    pub session_backend: SessionBackend,

    /// Functionality for sending emails
    pub email: EmailSender,
    /// A list of hosts that the server is listening on
    pub hosts: Vec<String>,

    /// An HTTP client for the server to make requests with. The client is shared so that they will
    /// all use the same request pool, and allow other things like custom User-Agent across the
    /// server.
    pub http_client: reqwest::Client,

    /// Control behavior around adding new users
    #[cfg(feature = "local_auth")]
    pub new_user_flags: NewUserFlags,

    #[cfg(feature = "local_auth")]
    /// Functionality for creating users in the app using Filigree
    pub user_creator: Box<dyn UserCreator>,

    #[cfg(feature = "local_auth")]
    /// The enabled OAuth Providers. This can be populated using [create_supported_providers].
    pub oauth_providers: Vec<Box<dyn OAuthProvider>>,

    /// Error reporting
    pub error_reporter: ErrorReporter,
}

impl FiligreeState {
    /// Return the default host from the configured list, stripped of wildcards if it includes one
    pub fn default_host(&self) -> &str {
        let mut default_host = self.hosts[0].as_str();
        if default_host.starts_with("*.") {
            default_host = &default_host[2..];
        }

        default_host
    }

    /// Check if a host is in the allowed list. If so, return the host. If not, return
    /// the first host in the list inside an `Err`.
    pub fn host_is_allowed<'a>(&'a self, host: &'a str) -> Result<&'a str, &'a str> {
        if self.hosts.is_empty() {
            return Ok(host);
        }

        let allowed = self.hosts.iter().any(|h| {
            if h.starts_with("*.") {
                host.ends_with(h)
            } else {
                host == h
            }
        });

        if allowed {
            Ok(host)
        } else {
            Err(self.default_host())
        }
    }

    /// If a host is passed, check if it's in the allow list and return it if so.
    /// Otherwise, return the default host.
    pub fn get_valid_host<'a>(&'a self, host: Option<&'a Host>) -> &'a str {
        if let Some(Host(host)) = host {
            match self.host_is_allowed(host) {
                Ok(h) => h,
                // The error is the default host
                Err(h) => h,
            }
        } else {
            self.default_host()
        }
    }
}

#[cfg(feature = "local_auth")]
/// Flags controlling new user behavior
pub struct NewUserFlags {
    /// Allow anyone to sign up
    pub allow_public_signup: bool,
    /// Allow inviting users to join your own organization
    pub allow_invite_to_same_org: bool,
    /// When inviting a new user to your organization, require email verification first.
    pub same_org_invites_require_email_verification: bool,
    /// Allow inviting new users into their own new organization.
    pub allow_invite_to_new_org: bool,
}

/// Create a future which will resolve when receiving SIGINT or SIGTERM
pub async fn shutdown_signal() {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c().await.expect("failed to listen for ctrl+c");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        event!(Level::INFO, "Shutting down server...");
        shutdown_tx.send(()).ok();
    });

    shutdown_rx.map(|_| ()).await
}
