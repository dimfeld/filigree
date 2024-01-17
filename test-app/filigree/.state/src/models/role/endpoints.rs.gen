#![allow(unused_imports, dead_code)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing, Json,
};

use super::{
    queries, types::*, RoleId, CREATE_PERMISSION, OWNER_PERMISSION, READ_PERMISSION,
    WRITE_PERMISSION,
};
use crate::{
    auth::{has_any_permission, Authed},
    server::ServerState,
    Error,
};

async fn get(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<RoleId>,
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
    Json(payload): Json<RoleCreatePayload>,
) -> Result<impl IntoResponse, Error> {
    let result = queries::create(&state.db, &auth, &payload).await?;

    Ok((StatusCode::CREATED, Json(result)))
}

async fn update(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<RoleId>,
    Json(payload): Json<RoleUpdatePayload>,
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
    Path(id): Path<RoleId>,
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
        .route(
            "/roles",
            routing::get(list).route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/roles/:id",
            routing::get(get).route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/roles",
            routing::post(create)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/roles/:id",
            routing::put(update).route_layer(has_any_permission(vec![
                WRITE_PERMISSION,
                OWNER_PERMISSION,
                "org_admin",
            ])),
        )
        .route(
            "/roles/:id",
            routing::delete(delete)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
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

    fn make_create_payload(i: usize) -> RoleCreatePayload {
        RoleCreatePayload {
            name: format!("Test object {i}"),
            description: (i > 1).then(|| format!("Test object {i}")),
        }
    }

    async fn setup_test_objects(
        db: &sqlx::PgPool,
        organization_id: OrganizationId,
        count: usize,
    ) -> Vec<Role> {
        futures::stream::iter(1..=count)
            .map(Ok)
            .and_then(|i| async move {
                let id = RoleId::new();
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

                admin_role,
                user_role,
                ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 3).await;

        let results = admin_user
            .client
            .get("roles")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap();

        let fixed_roles = [admin_role.to_string(), user_role.to_string()];
        let results = results
            .into_iter()
            .filter(|value| {
                !fixed_roles
                    .iter()
                    .any(|i| i == value["id"].as_str().unwrap())
            })
            .collect::<Vec<_>>();

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
                result["description"],
                serde_json::to_value(&added.description).unwrap(),
                "field description"
            );
            assert_eq!(result["_permission"], "owner");

            // Check that we don't return any fields which are supposed to be omitted.
        }

        let results = user
            .client
            .get("roles")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap();

        let fixed_roles = [admin_role.to_string(), user_role.to_string()];
        let results = results
            .into_iter()
            .filter(|value| {
                !fixed_roles
                    .iter()
                    .any(|i| i == value["id"].as_str().unwrap())
            })
            .collect::<Vec<_>>();

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
                result["description"],
                serde_json::to_value(&added.description).unwrap(),
                "field description"
            );
            assert_eq!(result["_permission"], "write");

            // Check that we don't return any fields which are supposed to be omitted.
        }

        let response = no_roles_user.client.get("roles").send().await.unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
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
                user,
                no_roles_user,
                ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 2).await;

        let result = admin_user
            .client
            .get(&format!("roles/{}", added_objects[1].id))
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
            result["description"],
            serde_json::to_value(&added.description).unwrap(),
            "field description"
        );
        assert_eq!(result["_permission"], "owner");

        // Check that we don't return any fields which are supposed to be omitted.

        let result = user
            .client
            .get(&format!("roles/{}", added_objects[1].id))
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
            result["description"],
            serde_json::to_value(&added.description).unwrap(),
            "field description"
        );
        assert_eq!(result["_permission"], "write");

        // Check that we don't return any fields which are supposed to be omitted.

        let response = no_roles_user
            .client
            .get(&format!("roles/{}", added_objects[1].id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
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
        let update_payload = RoleUpdatePayload {
            name: format!("Test object {i}"),

            description: Some(format!("Test object {i}")),
        };

        admin_user
            .client
            .put(&format!("roles/{}", added_objects[1].id))
            .json(&update_payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let updated: serde_json::Value = admin_user
            .client
            .get(&format!("roles/{}", added_objects[1].id))
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
            updated["description"],
            serde_json::to_value(&update_payload.description).unwrap(),
            "field description"
        );
        assert_eq!(updated["_permission"], "owner");

        // TODO Test that owner can not write fields which are not writable by anyone.
        // TODO Test that user can not update fields which are writable by owner but not user

        // Make sure that no other objects were updated
        let non_updated: serde_json::Value = admin_user
            .client
            .get(&format!("roles/{}", added_objects[0].id))
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
            non_updated["description"],
            serde_json::to_value(&added_objects[0].description).unwrap(),
            "field description"
        );
        assert_eq!(non_updated["_permission"], "owner");

        let response = no_roles_user
            .client
            .put(&format!("roles/{}", added_objects[1].id))
            .json(&update_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[sqlx::test]
    async fn create_object(pool: sqlx::PgPool) {
        let (
            _app,
            BootstrappedData {
                admin_user,
                no_roles_user,
                ..
            },
        ) = start_app(pool.clone()).await;

        let create_payload = make_create_payload(10);
        let created_result: serde_json::Value = admin_user
            .client
            .post("roles")
            .json(&create_payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            created_result["name"],
            serde_json::to_value(&create_payload.name).unwrap(),
            "field name from create response"
        );
        assert_eq!(
            created_result["description"],
            serde_json::to_value(&create_payload.description).unwrap(),
            "field description from create response"
        );
        assert_eq!(created_result["_permission"], "owner");

        let created_id = created_result["id"].as_str().unwrap();
        let get_result = admin_user
            .client
            .get(&format!("roles/{}", created_id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();

        assert_eq!(
            get_result["id"], created_result["id"],
            "field id from get response"
        );
        assert_eq!(
            get_result["organization_id"], created_result["organization_id"],
            "field organization_id from get response"
        );
        assert_eq!(
            get_result["updated_at"], created_result["updated_at"],
            "field updated_at from get response"
        );
        assert_eq!(
            get_result["created_at"], created_result["created_at"],
            "field created_at from get response"
        );
        assert_eq!(
            get_result["name"], created_result["name"],
            "field name from get response"
        );
        assert_eq!(
            get_result["description"], created_result["description"],
            "field description from get response"
        );
        assert_eq!(get_result["_permission"], "owner");

        let response = no_roles_user
            .client
            .post("roles")
            .json(&create_payload)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
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
            .delete(&format!("roles/{}", added_objects[1].id))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let response = admin_user
            .client
            .get(&format!("roles/{}", added_objects[1].id))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        // Delete should not happen without permissions
        let response = no_roles_user
            .client
            .delete(&format!("roles/{}", added_objects[0].id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

        // Make sure other objects still exist
        let response = admin_user
            .client
            .get(&format!("roles/{}", added_objects[0].id))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }
}
