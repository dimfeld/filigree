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
