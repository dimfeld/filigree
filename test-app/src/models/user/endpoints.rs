#![allow(unused_imports, dead_code)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing, Json,
};

use super::{queries, types::*, UserId, OWNER_PERMISSION};
use crate::{
    auth::{has_permission, Authed},
    server::ServerState,
    Error,
};

async fn get(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<UserId>,
) -> Result<impl IntoResponse, Error> {
    let object = queries::get(&state.db, &auth, id).await?;
    Ok(Json(object))
}

async fn list(
    State(state): State<ServerState>,
    auth: Authed,
    Query(qs): Query<queries::ListQueryFilters>,
) -> Result<impl IntoResponse, Error> {
    let results = queries::list(&state.db, &auth, &qs).await?;
    Ok(Json(results))
}

async fn create(
    State(state): State<ServerState>,
    auth: Authed,
    Json(payload): Json<UserCreatePayload>,
) -> Result<impl IntoResponse, Error> {
    let result = queries::create(&state.db, &auth, &payload).await?;

    Ok((StatusCode::CREATED, Json(result)))
}

async fn update(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<UserId>,
    Json(payload): Json<UserUpdatePayload>,
) -> Result<impl IntoResponse, Error> {
    queries::update(&state.db, &auth, id, &payload).await?;
    Ok(StatusCode::OK)
}

async fn delete(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<UserId>,
) -> Result<impl IntoResponse, Error> {
    queries::delete(&state.db, &auth, id).await?;

    Ok(StatusCode::OK)
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/user", routing::get(list))
        .route("/user/:id", routing::get(get))
        .route("/user/:id", routing::put(update))
        .route("/user/:id", routing::delete(delete))
}

#[cfg(test)]
mod test {
    use futures::{StreamExt, TryStreamExt};
    use tracing::{event, Level};

    use super::*;
    use crate::{
        models::organization::OrganizationId,
        tests::{start_app, BootstrappedData},
    };

    async fn setup_test_objects(
        db: &sqlx::PgPool,
        organization_id: OrganizationId,
        count: usize,
    ) -> Vec<User> {
        futures::stream::iter(1..=count)
            .map(Ok)
            .and_then(|i| async move {
                let id = UserId::new();
                event!(Level::INFO, %id, "Creating test object {}", i);
                super::queries::create_raw(
                    db,
                    id,
                    organization_id,
                    &UserCreatePayload {
                        name: format!("Test object {i}"),
                        email: format!("Test object {i}"),
                        verified: i % 2 == 0,
                    },
                )
                .await
            })
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn list_objects(pool: sqlx::PgPool) {
        let (
            _app,
            BootstrappedData {
                organization,
                admin_user,
                user,
                admin_role,
                user_role,
                ..
            },
        ) = start_app(pool.clone()).await;

        let mut added_objects = setup_test_objects(&pool, organization.id, 3).await;

        let mut results = admin_user
            .client
            .get("user")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap();

        let fixed_users = [admin_user.user_id.to_string(), user.user_id.to_string()];
        results.retain_mut(|value| {
            !fixed_users
                .iter()
                .any(|i| i == value["id"].as_str().unwrap())
        });

        assert_eq!(results.len(), added_objects.len());

        for result in results {
            let added = added_objects
                .iter()
                .find(|i| i.id.to_string() == result["id"].as_str().unwrap())
                .expect("Returned object did not match any of the added objects");

            assert_eq!(serde_json::to_value(&added.id).unwrap(), result["id"]);

            assert_eq!(
                serde_json::to_value(&added.organization_id).unwrap(),
                result["organization_id"]
            );

            assert_eq!(
                serde_json::to_value(&added.updated_at).unwrap(),
                result["updated_at"]
            );

            assert_eq!(
                serde_json::to_value(&added.created_at).unwrap(),
                result["created_at"]
            );

            assert_eq!(serde_json::to_value(&added.name).unwrap(), result["name"]);

            assert_eq!(serde_json::to_value(&added.email).unwrap(), result["email"]);

            assert_eq!(
                serde_json::to_value(&added.verified).unwrap(),
                result["verified"]
            );
        }

        // TODO Add test for user with only "read" permission and make sure that fields that are
        // owner_read but not user_read are omitted.
    }

    #[sqlx::test]
    #[ignore = "todo"]
    async fn list_fetch_specific_ids(_pool: sqlx::PgPool) {}

    #[sqlx::test]
    #[ignore = "todo"]
    async fn list_order_by(_pool: sqlx::PgPool) {}

    #[sqlx::test]
    #[ignore = "todo"]
    async fn list_paginated(_pool: sqlx::PgPool) {}

    #[sqlx::test]
    #[ignore = "todo"]
    async fn list_filters(_pool: sqlx::PgPool) {}

    #[sqlx::test]
    #[ignore = "todo"]
    async fn get_object(_pool: sqlx::PgPool) {}

    #[sqlx::test]
    #[ignore = "todo"]
    async fn update_object(_pool: sqlx::PgPool) {}

    #[sqlx::test]
    #[ignore = "todo"]
    async fn delete_object(_pool: sqlx::PgPool) {}
}
