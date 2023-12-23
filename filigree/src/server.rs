use tokio::signal;

/// Create a channel which will close when receiving SIGINT or SIGTERM
pub async fn shutdown_signal() -> tokio::sync::oneshot::Receiver<()> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    tokio::task::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c().await.expect("failed to listen for ctrl+c");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
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

    shutdown_rx
}
