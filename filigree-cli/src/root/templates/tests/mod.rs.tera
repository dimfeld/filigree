use error_stack::Report;
use filigree::{
    auth::{ExpiryStyle, SessionBackend, SessionCookieBuilder},
    testing::{self, TestClient, TestUser},
};
use futures::future::FutureExt;
use sqlx::PgPool;

use crate::{
    models::{organization, role, user},
    users::organization::{create_new_organization, CreatedOrganization},
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

pub struct BootstrappedData {
    organization: Organization,
    admin_role: RoleId,
    user_role: RoleId,
    admin_user: TestUser,
    user: TestUser,
}

pub async fn start_app(pg_pool: PgPool) -> TestApp {
    filigree::tracing_config::test::init();

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

    let bootstrapped_data = bootstrap_data(&pg_pool & test_client).await;

    let server_task = tokio::task::spawn(server.run_with_shutdown_signal(shutdown_rx));

    TestApp {
        shutdown_tx,
        client: test_client,
        bootstrapped_data,
        base_url,
        server_task,
        pg_pool,
    }
}

async fn add_test_api_key(
    db: impl PgExecutor<'_>,
    organization_id: OrganizationId,
    user_id: UserId,
) -> Result<String> {
    let key_data = ApiKeyData::new();
}

async fn add_test_user(
    db: &mut PgConnection,
    base_client: &TestClient,
    user_id: UserId,
    organization_id: OrganizationId,
    name: &str,
) -> Result<TestUser> {
    let key_data = ApiKeyData::new();

    let test_client = base_client.with_api_key(&key_data.key);

    let email = format!("{name}@example.com");
    let user_payload = user::UserCreatePayload {
        email: email.clone(),
        name: name.to_string(),
        password_hash: testing::TEST_PASSWORD_HASH.into(),
        ..Default::default()
    };

    user::queries::create_raw(db, user_id, organization_id, &user_payload)
        .await
        .expect("Creating user");

    let key = filigree::auth::api_key::ApiKey {
        api_key_id: key_data.api_key_id,
        organization_id,
        user_id: Some(user_id),
        inherits_user_permissions: true,
        description: String::new(),
        active: true,
        expires_at: Utc::now() + chrono::Duration::days(365),
    };
    filigree::auth::api_key::add_api_key(db, &key, &key_data.hash)
        .await
        .expect("Adding api key");

    filigree::users::users::add_user_email_login(&mut *tx, user_id, email, true)
        .await
        .expect("Adding admin email login");

    TestUser {
        user_id,
        organization_id,
        password: testing::TEST_PASSWORD.to_string(),
        client: test_client,
        api_key: key_data.key,
    }
}

async fn bootstrap_data(pg_pool: &sqlx::PgPool, base_client: &TestClient) {
    let mut tx = pg_pool.begin().await.unwrap();
    let admin_user_id = testing::ADMIN_USER_ID;
    let CreatedOrganization {
        organization,
        user_role,
        admin_role,
    } = crate::users::organization::create_new_organization(
        &mut *tx,
        "Test Org".into(),
        admin_user_id,
    )
    .await
    .expect("Creating test org");

    let admin_user = add_test_user(&mut *tx, admin_user_id, organization.id, "Admin")
        .await
        .expect("Creating admin user");
    let regular_user = add_test_user(&mut *tx, UserId::new(), organization.id, "User")
        .await
        .expect("Creating regular user");
    filigree::users::roles::add_roles_to_user(
        &mut *tx,
        organization.id,
        regular_user.user_id,
        &[user_role],
    )
    .await
    .expect("Adding user role to regular user");

    tx.commit().await.unwrap();

    BootstrappedData {
        organization,
        user_role,
        admin_role,
        admin_user,
        user: regular_user,
    }
}
