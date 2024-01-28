use futures::FutureExt;
use tokio::signal;

use crate::{auth::SessionBackend, email::services::EmailSender, users::users::UserCreator};

/// Internal state used by the server
pub struct FiligreeState {
    /// The database connection pool
    pub db: sqlx::PgPool,
    /// User session backend
    pub session_backend: SessionBackend,
    /// Functionality for sending emails
    pub email: EmailSender,
    /// A list of hosts that the server is listening on
    pub hosts: Vec<String>,

    pub http_client: reqwest::Client,

    /// Control behavior around adding new users
    pub new_user_flags: NewUserFlags,

    pub user_creator: Box<dyn UserCreator>,
}

impl FiligreeState {
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
            let mut default_host = self.hosts[0].as_str();
            if default_host.starts_with("*.") {
                default_host = &default_host[2..];
            }
            Err(default_host)
        }
    }
}

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

        shutdown_tx.send(()).ok();
    });

    shutdown_rx.map(|_| ()).await
}
