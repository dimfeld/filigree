use futures::FutureExt;
use tokio::signal;

use crate::{auth::SessionBackend, email::services::EmailSender};

/// Internal state used by the server
pub struct FiligreeState {
    /// The database connection pool
    pub db: sqlx::PgPool,
    /// User session backend
    pub session_backend: SessionBackend,
    /// Functionality for sending emails
    pub email: EmailSender,

    /// Control behavior around adding new users
    pub new_user_flags: NewUserFlags,
}

/// Flags controlling new user behavior
pub struct NewUserFlags {
    /// Require users to verify their email address before they can use the site.
    pub require_email_verification: bool,
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
