use error_stack::Report;
use filigree::testing::{TestClient, TestUser, TEST_PASSWORD};

use crate::Error;

pub struct TestApp {
    /// Hold on to the shutdown signal so the server stays alive
    _shutdown_tx: tokio::sync::oneshot::Sender<()>,
    client: TestClient,
    admin_user: TestUser,
    base_url: String,
    server_task: tokio::task::JoinHandle<Result<(), Report<Error>>>,
}

pub async fn start_app(pg_pool: sqlx::PgPool) {
    filigree::tracing_config::test::init();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    // Make the shutdown future resolve to () so the type matches what Axum expects.
    let shutdown_rx = shutdown_rx.map(|_| ());

    let config = crate::server::Config {
        env: "test".into(),
        host: Some("127.0.0.1".into()),
        port: 0, // Bind to random port
        request_timeout: std::Duration::from_secs(30),
        pg_pool,
        cookie_configuration: SessionCookieBuilder::new(
            false,
            tower_cookies::cookie::SameSite::Strict,
        ),
        session_expiry: ExpiryStyle::AfterIdle(std::time::Duration::from_secs(24 * 60 * 60)),
    };

    let server = crate::server::create_server(config)
        .await
        .expect("creating server");

    let base_url = format!("http://{}:{}", server.host, server.port);
    let test_client = TestClient::new(base_url.clone());

    let admin_user = TestUser {
        user_id,
        organization_id,
        password: TEST_PASSWORD,
        client: test_client.with_api_key(&api_key),
        api_key,
    };

    let server_task = tokio::task::spawn(server.run_with_shutdown_signal(shutdown_rx));

    TestApp {
        _shutdown_tx: shutdown_tx,
        client: test_client,
        admin_user,
        base_url,
        server_task,
    }
}
