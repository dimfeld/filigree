use error_stack::{Report, ResultExt};
use sqlx::PgExecutor;

use crate::{
    models::{
        organization::OrganizationId,
        user::{User, UserCreatePayload, UserId},
    },
    Error,
};

pub async fn create_new_user(
    db: impl PgExecutor<'_>,
    user_id: UserId,
    organization_id: OrganizationId,
    payload: UserCreatePayload,
    password_plaintext: String,
) -> Result<User, Report<Error>> {
    let password_hash = filigree::auth::password::new_hash(password_plaintext)
        .await
        .change_context(Error::AuthSubsystem)?;

    create_new_user_with_prehashed_password(db, user_id, organization_id, payload, password_hash)
        .await
}

pub async fn create_new_user_with_prehashed_password(
    db: impl PgExecutor<'_>,
    user_id: UserId,
    organization_id: OrganizationId,
    payload: UserCreatePayload,
    password_hash: String,
) -> Result<User, Report<Error>> {
    let user = sqlx::query_file_as!(
        User,
        "src/users/create_user.sql",
        user_id.as_uuid(),
        organization_id.as_uuid(),
        password_hash,
        &payload.name,
        &payload.email,
        &payload.verified,
    )
    .fetch_one(db)
    .await
    .change_context(Error::Db)?;

    Ok(user)
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::tests::{start_app, BootstrappedData};

    #[sqlx::test]
    async fn login_with_password_and_logout(db: sqlx::PgPool) {
        let (app, BootstrappedData { admin_user, .. }) = start_app(db).await;

        let client = &app.client;
        let response: serde_json::Value = client
            .post("login")
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
            .get(&format!("user/{}", admin_user.user_id))
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
            .post("logout")
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
            .get(&format!("user/{}", admin_user.user_id))
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
    async fn login_with_no_roles_user(db: sqlx::PgPool) {
        let (app, BootstrappedData { no_roles_user, .. }) = start_app(db).await;

        let client = &app.client;
        let response: serde_json::Value = client
            .post("login")
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
            .get(&format!("user/{}", no_roles_user.user_id))
            .send()
            .await
            .unwrap();
        // Should see 404 because user has no roles and hence no permissions, but not
        // 401 which would indicate that the lack of roles is causing the user lookup query to
        // fail.
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    }
}
