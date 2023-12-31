#![allow(unused_imports, dead_code)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing, Json,
};

use super::{queries, types::*, UserId, OWNER_PERMISSION};
use crate::{
    auth::{has_any_permission, Authed},
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
    let updated = queries::update(&state.db, &auth, id, &payload).await?;
    let status = if updated {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    Ok(status)
}

async fn delete(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<UserId>,
) -> Result<impl IntoResponse, Error> {
    let deleted = queries::delete(&state.db, &auth, id).await?;

    let status = if deleted {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    Ok(status)
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

    fn make_create_payload(i: usize) -> UserCreatePayload {
        UserCreatePayload {
            name: format!("Test object {i}"),
            email: format!("Test object {i}"),
        }
    }

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
                super::queries::create_raw(db, id, organization_id, &make_create_payload(i)).await
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
                no_roles_user,

                user,
                ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 3).await;

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

        let fixed_users = [
            admin_user.user_id.to_string(),
            user.user_id.to_string(),
            no_roles_user.user_id.to_string(),
        ];
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
            assert_eq!(
                result["id"],
                serde_json::to_value(&added.id).unwrap(),
                "field id"
            );
            assert_eq!(
                result["organization_id"],
                serde_json::to_value(&added.organization_id).unwrap(),
                "field organization_id"
            );
            assert_eq!(
                result["updated_at"],
                serde_json::to_value(&added.updated_at).unwrap(),
                "field updated_at"
            );
            assert_eq!(
                result["created_at"],
                serde_json::to_value(&added.created_at).unwrap(),
                "field created_at"
            );
            assert_eq!(
                result["name"],
                serde_json::to_value(&added.name).unwrap(),
                "field name"
            );
            assert_eq!(
                result["email"],
                serde_json::to_value(&added.email).unwrap(),
                "field email"
            );
            assert_eq!(
                result["verified"],
                serde_json::to_value(&added.verified).unwrap(),
                "field verified"
            );
            assert_eq!(result["_permission"], "owner");

            // Check that we don't return any fields which are supposed to be omitted.
            assert_eq!(
                result.get("password_hash"),
                None,
                "field password_hash should be omitted"
            );
        }

        // TODO Add test for user with only "read" permission and make sure that fields that are
        // owner_read but not user_read are omitted.

        let response: Vec<serde_json::Value> = no_roles_user
            .client
            .get("user")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert!(response.is_empty());
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
    async fn get_object(pool: sqlx::PgPool) {
        let (
            _app,
            BootstrappedData {
                organization,
                admin_user,
                no_roles_user,
                ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 2).await;

        let result = admin_user
            .client
            .get(&format!("user/{}", added_objects[1].id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();

        let added = &added_objects[1];
        assert_eq!(
            result["id"],
            serde_json::to_value(&added.id).unwrap(),
            "field id"
        );
        assert_eq!(
            result["organization_id"],
            serde_json::to_value(&added.organization_id).unwrap(),
            "field organization_id"
        );
        assert_eq!(
            result["updated_at"],
            serde_json::to_value(&added.updated_at).unwrap(),
            "field updated_at"
        );
        assert_eq!(
            result["created_at"],
            serde_json::to_value(&added.created_at).unwrap(),
            "field created_at"
        );
        assert_eq!(
            result["name"],
            serde_json::to_value(&added.name).unwrap(),
            "field name"
        );
        assert_eq!(
            result["email"],
            serde_json::to_value(&added.email).unwrap(),
            "field email"
        );
        assert_eq!(
            result["verified"],
            serde_json::to_value(&added.verified).unwrap(),
            "field verified"
        );
        assert_eq!(result["_permission"], "owner");

        // Check that we don't return any fields which are supposed to be omitted.
        assert_eq!(
            result.get("password_hash"),
            None,
            "field password_hash should be omitted"
        );

        // TODO Add test for user with only "read" permission and make sure that fields that are
        // owner_read but not user_read are omitted.

        let response = no_roles_user
            .client
            .get(&format!("user/{}", added_objects[1].id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn update_object(pool: sqlx::PgPool) {
        let (
            _app,
            BootstrappedData {
                organization,
                admin_user,
                no_roles_user,
                ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 2).await;

        let i = 20;
        let update_payload = UserUpdatePayload {
            name: format!("Test object {i}"),

            email: format!("Test object {i}"),
        };

        admin_user
            .client
            .put(&format!("user/{}", added_objects[1].id))
            .json(&update_payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let updated: serde_json::Value = admin_user
            .client
            .get(&format!("user/{}", added_objects[1].id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            updated["name"],
            serde_json::to_value(&update_payload.name).unwrap(),
            "field name"
        );
        assert_eq!(
            updated["email"],
            serde_json::to_value(&update_payload.email).unwrap(),
            "field email"
        );
        assert_eq!(updated["_permission"], "owner");

        // Make sure that no other objects were updated
        let non_updated: serde_json::Value = admin_user
            .client
            .get(&format!("user/{}", added_objects[0].id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            non_updated["id"],
            serde_json::to_value(&added_objects[0].id).unwrap(),
            "field id"
        );
        assert_eq!(
            non_updated["organization_id"],
            serde_json::to_value(&added_objects[0].organization_id).unwrap(),
            "field organization_id"
        );
        assert_eq!(
            non_updated["updated_at"],
            serde_json::to_value(&added_objects[0].updated_at).unwrap(),
            "field updated_at"
        );
        assert_eq!(
            non_updated["created_at"],
            serde_json::to_value(&added_objects[0].created_at).unwrap(),
            "field created_at"
        );
        assert_eq!(
            non_updated["name"],
            serde_json::to_value(&added_objects[0].name).unwrap(),
            "field name"
        );
        assert_eq!(
            non_updated["email"],
            serde_json::to_value(&added_objects[0].email).unwrap(),
            "field email"
        );
        assert_eq!(
            non_updated["verified"],
            serde_json::to_value(&added_objects[0].verified).unwrap(),
            "field verified"
        );
        assert_eq!(non_updated["_permission"], "owner");

        let response = no_roles_user
            .client
            .put(&format!("user/{}", added_objects[1].id))
            .json(&update_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn delete_object(pool: sqlx::PgPool) {
        let (
            _app,
            BootstrappedData {
                organization,
                admin_user,
                no_roles_user,
                ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 2).await;

        admin_user
            .client
            .delete(&format!("user/{}", added_objects[1].id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let response = admin_user
            .client
            .get(&format!("user/{}", added_objects[1].id))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        // Delete should not happen without permissions
        let response = no_roles_user
            .client
            .delete(&format!("user/{}", added_objects[0].id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        // Make sure other objects still exist
        let response = admin_user
            .client
            .get(&format!("user/{}", added_objects[0].id))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }
}
