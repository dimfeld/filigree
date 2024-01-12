use serde_json::json;

use crate::tests::{start_app, BootstrappedData};

pub fn extract_token_from_email(email: &filigree::email::Email) -> &str {
    email
        .text
        .split_once("token=")
        .unwrap()
        .1
        .split_once('&')
        .unwrap()
        .0
}

#[sqlx::test]
async fn login_with_password_and_logout(db: sqlx::PgPool) {
    let (app, BootstrappedData { admin_user, .. }) = start_app(db).await;

    let client = &app.client;
    let response: serde_json::Value = client
        .post("auth/login")
        .json(&json!({ "email": admin_user.email, "password": admin_user.password }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["message"], "Logged in");

    let user: serde_json::Value = client
        .get(&format!("users/{}", admin_user.user_id))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(user["name"], "Admin");

    let response: serde_json::Value = client
        .post("auth/logout")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response["message"], "Logged out");

    let anon_response = client
        .get(&format!("users/{}", admin_user.user_id))
        .send()
        .await
        .unwrap();

    assert_eq!(
        anon_response.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "Authed requests should not work after logout"
    );

    // TODO check explicitly that the session cookie is gone
    // TODO check that adding the session cookie back to the request after logout doesn't work
}

#[sqlx::test]
async fn login_with_nonexistent_email(db: sqlx::PgPool) {
    let (app, BootstrappedData { admin_user, .. }) = start_app(db).await;

    let client = &app.client;
    let response = client
        .post("auth/login")
        .json(&json!({ "email": "nobody@example.com", "password": admin_user.password }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn login_with_wrong_password(db: sqlx::PgPool) {
    let (app, BootstrappedData { admin_user, .. }) = start_app(db).await;

    let client = &app.client;
    let response = client
        .post("auth/login")
        .json(&json!({ "email": admin_user.email, "password": "wrong" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn login_with_no_roles_user(db: sqlx::PgPool) {
    let (app, BootstrappedData { no_roles_user, .. }) = start_app(db).await;

    let client = &app.client;
    let response: serde_json::Value = client
        .post("auth/login")
        .json(&json!({ "email": no_roles_user.email, "password": no_roles_user.password }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["message"], "Logged in");

    let response = client
        .get(&format!("users/{}", no_roles_user.user_id))
        .send()
        .await
        .unwrap();
    // Should see 403 because user has no roles and hence no permissions, but not
    // 401 which would indicate some other problem in the auth system.
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
}
