use error_stack::Report;
use filigree::{
    auth::{ExpiryStyle, SessionBackend, SessionCookieBuilder},
    testing::{self, TestClient, TestUser},
};
use futures::future::FutureExt;
use sqlx::PgPool;

use crate::{
    models::{organization, role, user},
    Error,
};

pub struct TestApp {
    /// Hold on to the shutdown signal so the server stays alive
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
    pub client: TestClient,
    // pub admin_user: TestUser,
    pub base_url: String,
    pub pg_pool: PgPool,
    pub server_task: tokio::task::JoinHandle<Result<(), Report<Error>>>,
}

pub async fn start_app(pg_pool: PgPool) -> TestApp {
    filigree::tracing_config::test::init();

    bootstrap_data(&pg_pool).await;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    // Make the shutdown future resolve to () so the type matches what Axum expects.
    let shutdown_rx = shutdown_rx.map(|_| ());

    let config = crate::server::Config {
        env: "test".into(),
        host: "127.0.0.1".into(),
        port: 0, // Bind to random port
        request_timeout: std::time::Duration::from_secs(30),
        pg_pool: pg_pool.clone(),
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

    // let admin_user = TestUser {
    //     user_id: testing::ADMIN_USER_ID,
    //     organization_id: testing::MAIN_ORG_ID,
    //     password: testing::TEST_PASSWORD.to_string(),
    //     client: test_client.with_api_key(&api_key),
    //     api_key,
    // };

    let server_task = tokio::task::spawn(server.run_with_shutdown_signal(shutdown_rx));

    TestApp {
        shutdown_tx,
        client: test_client,
        // admin_user,
        base_url,
        server_task,
        pg_pool,
    }
}

async fn bootstrap_data(pg_pool: &sqlx::PgPool) {
    // Create org
    // Create admin user
    // Update organization_members
    // Update permissions
    // Update email_logins
    // Update api_keys

    // Add these functions in the main code:
    //  Create org
    //  Create roles
    //  Create user and add to org, along with permissions and roles
    //  Add api key for user
}
