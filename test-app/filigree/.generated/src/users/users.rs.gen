use axum::{extract::State, http::StatusCode, response::IntoResponse, routing, Json};
use error_stack::{Report, ResultExt};
use sqlx::PgExecutor;

use crate::{
    auth::Authed,
    models::{
        organization::OrganizationId,
        user::{User, UserCreatePayload, UserId},
    },
    server::ServerState,
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

    create_new_user_with_prehashed_password(
        db,
        user_id,
        organization_id,
        payload,
        Some(password_hash),
    )
    .await
}

pub async fn create_new_user_with_prehashed_password(
    db: impl PgExecutor<'_>,
    user_id: UserId,
    organization_id: OrganizationId,
    payload: UserCreatePayload,
    password_hash: Option<String>,
) -> Result<User, Report<Error>> {
    let user = sqlx::query_file_as!(
        User,
        "src/users/create_user.sql",
        user_id.as_uuid(),
        organization_id.as_uuid(),
        password_hash,
        &payload.name,
        &payload.email,
    )
    .fetch_one(db)
    .await
    .change_context(Error::Db)?;

    Ok(user)
}

async fn get_current_user_endpoint(
    State(state): State<ServerState>,
    authed: Authed,
) -> Result<impl IntoResponse, Error> {
    // TODO This probably should be a more custom query, include organization info and permissions
    // and such, and work even if the user doesn't have the User:read permission.
    let user = crate::models::user::queries::get(&state.db, &authed, authed.user_id).await?;

    Ok(Json(user))
}

async fn update_current_user_endpoint(
    State(state): State<ServerState>,
    authed: Authed,
    Json(body): Json<crate::models::user::UserUpdatePayload>,
) -> Result<impl IntoResponse, Error> {
    // TODO Need a permission specifically for updating self
    let updated =
        crate::models::user::queries::update(&state.db, &authed, authed.user_id, &body).await?;

    let status = if updated {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    Ok(status)
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/self", routing::get(get_current_user_endpoint))
        .route("/self", routing::put(update_current_user_endpoint))
}

#[cfg(test)]
mod test {
    use crate::tests::{start_app, BootstrappedData};

    #[sqlx::test]
    async fn get_current_user(db: sqlx::PgPool) {
        let (app, BootstrappedData { admin_user, .. }) = start_app(db).await;

        let user_info: serde_json::Value = admin_user
            .client
            .get("self")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(user_info["name"], "Admin");
    }

    #[sqlx::test]
    async fn update_current_user(db: sqlx::PgPool) {
        let (app, BootstrappedData { admin_user, .. }) = start_app(db).await;

        let payload = crate::models::user::UserUpdatePayload {
            name: "Not Admin".into(),
            email: "another-email@example.com".into(),
            ..Default::default()
        };

        admin_user
            .client
            .put("self")
            .json(&payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let user_info: serde_json::Value = admin_user
            .client
            .get("self")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(user_info["name"], "Not Admin");
        assert_eq!(user_info["email"], "another-email@example.com");
    }
}
