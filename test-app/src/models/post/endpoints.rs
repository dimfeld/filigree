#![allow(unused_imports, dead_code)]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use axum_extra::extract::Query;
use axum_jsonschema::Json;
use error_stack::ResultExt;
use filigree::{auth::ObjectPermission, extract::FormOrJson};
use tracing::{event, Level};

use super::{
    queries, types::*, PostId, CREATE_PERMISSION, OWNER_PERMISSION, READ_PERMISSION,
    WRITE_PERMISSION,
};
use crate::{
    auth::{has_any_permission, Authed},
    models::{
        comment::{Comment, CommentCreatePayload, CommentId, CommentUpdatePayload},
        poll::{Poll, PollCreatePayload, PollId, PollUpdatePayload},
        post_image::{PostImage, PostImageCreatePayload, PostImageId, PostImageUpdatePayload},
        reaction::{Reaction, ReactionCreatePayload, ReactionId, ReactionUpdatePayload},
    },
    server::ServerState,
    Error,
};

async fn get(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<PostId>,
) -> Result<impl IntoResponse, Error> {
    let object = queries::get_populated(&state.db, &auth, id).await?;

    Ok(Json(object))
}

async fn list(
    State(state): State<ServerState>,
    auth: Authed,
    Query(qs): Query<queries::ListQueryFilters>,
) -> Result<impl IntoResponse, Error> {
    let results = queries::list_populated(&state.db, &auth, &qs).await?;

    Ok(Json(results))
}

async fn create(
    State(state): State<ServerState>,
    auth: Authed,
    FormOrJson(payload): FormOrJson<PostCreatePayload>,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;
    let result = queries::create(&mut *tx, &auth, payload).await?;
    tx.commit().await.change_context(Error::Db)?;

    Ok((StatusCode::CREATED, Json(result)))
}

async fn update(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<PostId>,
    FormOrJson(payload): FormOrJson<PostUpdatePayload>,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;

    let result = queries::update(&mut *tx, &auth, id, payload).await?;

    tx.commit().await.change_context(Error::Db)?;

    if result {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

async fn delete(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<PostId>,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;

    let post_image_files = crate::models::post_image::storage::get_storage_keys_by_parent_id(
        &state, &auth, &mut *tx, id,
    )
    .await?;

    let deleted = queries::delete(&mut *tx, &auth, id).await?;

    if !deleted {
        return Ok(StatusCode::NOT_FOUND);
    }

    tx.commit().await.change_context(Error::Db)?;

    for file in post_image_files {
        crate::models::post_image::storage::delete_by_key(&state, &file).await?;
    }

    Ok(StatusCode::OK)
}

async fn list_child_comment(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    Query(mut qs): Query<crate::models::comment::queries::ListQueryFilters>,
) -> Result<impl IntoResponse, Error> {
    qs.post_id = vec![parent_id];

    let object = crate::models::comment::queries::list(&state.db, &auth, &qs).await?;

    Ok(Json(object))
}

async fn create_child_comment(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    FormOrJson(mut payload): FormOrJson<CommentCreatePayload>,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;

    payload.post_id = parent_id;

    let result = crate::models::comment::queries::create(&mut *tx, &auth, payload).await?;

    tx.commit().await.change_context(Error::Db)?;

    Ok(Json(result))
}

async fn update_child_comment(
    State(state): State<ServerState>,
    auth: Authed,
    Path((parent_id, child_id)): Path<(PostId, CommentId)>,
    FormOrJson(mut payload): FormOrJson<CommentUpdatePayload>,
) -> Result<impl IntoResponse, Error> {
    payload.id = Some(child_id);
    payload.post_id = parent_id;

    let result = crate::models::comment::queries::update_one_with_parent(
        &state.db, &auth, true, // TODO get the right value here
        parent_id, child_id, payload,
    )
    .await?;

    Ok(Json(result))
}

async fn delete_child_comment(
    State(state): State<ServerState>,
    auth: Authed,
    Path((parent_id, child_id)): Path<(PostId, CommentId)>,
) -> Result<impl IntoResponse, Error> {
    crate::models::comment::queries::delete_with_parent(&state.db, &auth, parent_id, child_id)
        .await?;

    Ok(StatusCode::OK)
}

async fn list_child_reaction(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    Query(mut qs): Query<crate::models::reaction::queries::ListQueryFilters>,
) -> Result<impl IntoResponse, Error> {
    qs.post_id = vec![parent_id];

    let object = crate::models::reaction::queries::list(&state.db, &auth, &qs).await?;

    Ok(Json(object))
}

async fn create_child_reaction(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    FormOrJson(mut payload): FormOrJson<ReactionCreatePayload>,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;

    payload.post_id = parent_id;

    let result = crate::models::reaction::queries::create(&mut *tx, &auth, payload).await?;

    tx.commit().await.change_context(Error::Db)?;

    Ok(Json(result))
}

async fn update_child_reaction(
    State(state): State<ServerState>,
    auth: Authed,
    Path((parent_id, child_id)): Path<(PostId, ReactionId)>,
    FormOrJson(mut payload): FormOrJson<ReactionUpdatePayload>,
) -> Result<impl IntoResponse, Error> {
    payload.id = Some(child_id);
    payload.post_id = parent_id;

    let result = crate::models::reaction::queries::update_one_with_parent(
        &state.db, &auth, true, // TODO get the right value here
        parent_id, child_id, payload,
    )
    .await?;

    Ok(Json(result))
}

async fn delete_child_reaction(
    State(state): State<ServerState>,
    auth: Authed,
    Path((parent_id, child_id)): Path<(PostId, ReactionId)>,
) -> Result<impl IntoResponse, Error> {
    crate::models::reaction::queries::delete_with_parent(&state.db, &auth, parent_id, child_id)
        .await?;

    Ok(StatusCode::OK)
}

async fn list_child_poll(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    Query(mut qs): Query<crate::models::poll::queries::ListQueryFilters>,
) -> Result<impl IntoResponse, Error> {
    qs.post_id = vec![parent_id];

    let object = crate::models::poll::queries::list(&state.db, &auth, &qs).await?;

    let object = object.into_iter().next().ok_or(Error::NotFound("Poll"))?;

    Ok(Json(object))
}

async fn upsert_child_poll(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    FormOrJson(mut payload): FormOrJson<PollUpdatePayload>,
) -> Result<impl IntoResponse, Error> {
    payload.post_id = parent_id;

    let object_perm = queries::lookup_object_permissions(&state.db, &auth, parent_id)
        .await?
        .unwrap_or(ObjectPermission::Read);

    object_perm.must_be_writable(WRITE_PERMISSION)?;

    let result = crate::models::poll::queries::upsert_with_parent(
        &state.db,
        auth.organization_id,
        object_perm == ObjectPermission::Owner,
        parent_id,
        &payload,
    )
    .await?;

    Ok(Json(result))
}

async fn delete_child_poll(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
) -> Result<impl IntoResponse, Error> {
    let object_perm = queries::lookup_object_permissions(&state.db, &auth, parent_id)
        .await?
        .unwrap_or(ObjectPermission::Read);

    object_perm.must_be_writable(WRITE_PERMISSION)?;

    crate::models::poll::queries::delete_all_children_of_parent(
        &state.db,
        auth.organization_id,
        parent_id,
    )
    .await?;

    Ok(StatusCode::OK)
}

async fn list_child_post_image(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    Query(mut qs): Query<crate::models::post_image::queries::ListQueryFilters>,
) -> Result<impl IntoResponse, Error> {
    qs.post_id = vec![parent_id];

    let object = crate::models::post_image::queries::list(&state.db, &auth, &qs).await?;

    Ok(Json(object))
}

async fn create_child_post_image(
    State(state): State<ServerState>,
    auth: Authed,
    Path(parent_id): Path<PostId>,
    Query(qs): Query<filigree::storage::QueryFilename>,
    body: axum::body::Body,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;

    let result = crate::models::post_image::storage::upload_stream(
        &state,
        &auth,
        &mut *tx,
        parent_id,
        None,
        qs.filename.clone(),
        None,
        None,
        body.into_data_stream(),
    )
    .await?;

    tx.commit().await.change_context(Error::Db)?;

    Ok(Json(result))
}

async fn delete_child_post_image(
    State(state): State<ServerState>,
    auth: Authed,
    Path((parent_id, child_id)): Path<(PostId, PostImageId)>,
) -> Result<impl IntoResponse, Error> {
    let mut tx = state.db.begin().await.change_context(Error::Db)?;
    crate::models::post_image::storage::delete_by_id(&state, &auth, &mut *tx, parent_id, child_id)
        .await?;
    tx.commit().await.change_context(Error::Db)?;

    Ok(StatusCode::OK)
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route(
            "/posts",
            routing::get(list).route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id",
            routing::get(get).route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts",
            routing::post(create)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id",
            routing::put(update).route_layer(has_any_permission(vec![
                WRITE_PERMISSION,
                OWNER_PERMISSION,
                "org_admin",
            ])),
        )
        .route(
            "/posts/:id",
            routing::delete(delete)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/comments",
            routing::get(list_child_comment)
                .route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/comments",
            routing::post(create_child_comment)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/comments/:child_id",
            routing::put(update_child_comment).route_layer(has_any_permission(vec![
                WRITE_PERMISSION,
                OWNER_PERMISSION,
                "org_admin",
            ])),
        )
        .route(
            "/posts/:id/comments/:child_id",
            routing::delete(delete_child_comment)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/reactions",
            routing::get(list_child_reaction)
                .route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/reactions",
            routing::post(create_child_reaction)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/reactions/:child_id",
            routing::put(update_child_reaction).route_layer(has_any_permission(vec![
                WRITE_PERMISSION,
                OWNER_PERMISSION,
                "org_admin",
            ])),
        )
        .route(
            "/posts/:id/reactions/:child_id",
            routing::delete(delete_child_reaction)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/poll",
            routing::get(list_child_poll)
                .route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/poll",
            routing::post(upsert_child_poll)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/poll",
            routing::put(upsert_child_poll).route_layer(has_any_permission(vec![
                WRITE_PERMISSION,
                OWNER_PERMISSION,
                "org_admin",
            ])),
        )
        .route(
            "/posts/:id/poll",
            routing::delete(delete_child_poll)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/post_images",
            routing::get(list_child_post_image)
                .route_layer(has_any_permission(vec![READ_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/post_images",
            routing::post(create_child_post_image)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
        .route(
            "/posts/:id/post_images/:child_id",
            routing::delete(delete_child_post_image)
                .route_layer(has_any_permission(vec![CREATE_PERMISSION, "org_admin"])),
        )
}

#[cfg(test)]
mod test {
    use filigree::testing::ResponseExt;
    use futures::{StreamExt, TryStreamExt};
    use tracing::{event, Level};

    use super::{
        super::testing::{make_create_payload, make_update_payload},
        *,
    };
    use crate::{
        models::organization::OrganizationId,
        tests::{start_app, BootstrappedData},
    };

    async fn setup_test_objects(
        db: &sqlx::PgPool,
        organization_id: OrganizationId,
        count: usize,
    ) -> Vec<(PostCreatePayload, PostCreateResult)> {
        // TODO if this model belongs_to another, then create the parent object for it

        let mut tx = db.begin().await.unwrap();
        let mut objects = Vec::with_capacity(count);
        for i in 1..=count {
            let id = PostId::new();
            event!(Level::INFO, %id, "Creating test object {}", i);
            let payload = make_create_payload(i);
            let result = super::queries::create_raw(&mut *tx, id, organization_id, payload.clone())
                .await
                .expect("Creating test object failed");

            objects.push((payload, result));
        }

        tx.commit().await.unwrap();
        objects
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

        let results = admin_user
            .client
            .get("posts")
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap();

        assert_eq!(results.len(), added_objects.len());

        for result in results {
            let (payload, added) = added_objects
                .iter()
                .find(|i| i.1.id.to_string() == result["id"].as_str().unwrap())
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
                result["subject"],
                serde_json::to_value(&added.subject).unwrap(),
                "field subject"
            );

            assert_eq!(
                payload.subject, added.subject,
                "create result field subject"
            );
            assert_eq!(
                result["body"],
                serde_json::to_value(&added.body).unwrap(),
                "field body"
            );

            assert_eq!(payload.body, added.body, "create result field body");

            assert_eq!(result["_permission"], "owner");

            // Check that we don't return any fields which are supposed to be omitted.
        }

        let results = user
            .client
            .get("posts")
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap();

        for result in results {
            let (payload, added) = added_objects
                .iter()
                .find(|i| i.1.id.to_string() == result["id"].as_str().unwrap())
                .expect("Returned object did not match any of the added objects");
            assert_eq!(
                result["id"],
                serde_json::to_value(&added.id).unwrap(),
                "list result field id"
            );
            assert_eq!(
                result["organization_id"],
                serde_json::to_value(&added.organization_id).unwrap(),
                "list result field organization_id"
            );
            assert_eq!(
                result["updated_at"],
                serde_json::to_value(&added.updated_at).unwrap(),
                "list result field updated_at"
            );
            assert_eq!(
                result["created_at"],
                serde_json::to_value(&added.created_at).unwrap(),
                "list result field created_at"
            );
            assert_eq!(
                result["subject"],
                serde_json::to_value(&added.subject).unwrap(),
                "list result field subject"
            );
            assert_eq!(
                result["body"],
                serde_json::to_value(&added.body).unwrap(),
                "list result field body"
            );
            assert_eq!(result["_permission"], "write");

            let ids = serde_json::json!([]);

            assert_eq!(result["comment_ids"], ids, "field comment_ids");

            let ids = serde_json::json!(null);

            assert_eq!(result["poll_id"], ids, "field poll_id");

            // Check that we don't return any fields which are supposed to be omitted.
        }

        let response = no_roles_user.client.get("posts").send().await.unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

        // TODO test list endpoint of child comment objects

        // TODO test list endpoint of child reaction objects

        // TODO test list endpoint of child post_image objects
    }

    #[sqlx::test]
    async fn list_fetch_specific_ids(pool: sqlx::PgPool) {
        let (
            _app,
            BootstrappedData {
                organization, user, ..
            },
        ) = start_app(pool.clone()).await;

        let added_objects = setup_test_objects(&pool, organization.id, 3).await;

        let results = user
            .client
            .get("posts")
            .query(&[("id", added_objects[0].1.id), ("id", added_objects[2].1.id)])
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .any(|o| o["id"] == added_objects[0].1.id.to_string()));
        assert!(results
            .iter()
            .any(|o| o["id"] == added_objects[2].1.id.to_string()));
    }

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
            .get(&format!("posts/{}", added_objects[1].1.id))
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();

        let (payload, added) = &added_objects[1];
        assert_eq!(
            result["id"],
            serde_json::to_value(&added.id).unwrap(),
            "get result field id"
        );
        assert_eq!(
            result["organization_id"],
            serde_json::to_value(&added.organization_id).unwrap(),
            "get result field organization_id"
        );
        assert_eq!(
            result["updated_at"],
            serde_json::to_value(&added.updated_at).unwrap(),
            "get result field updated_at"
        );
        assert_eq!(
            result["created_at"],
            serde_json::to_value(&added.created_at).unwrap(),
            "get result field created_at"
        );
        assert_eq!(
            result["subject"],
            serde_json::to_value(&added.subject).unwrap(),
            "get result field subject"
        );

        assert_eq!(
            added.subject, payload.subject,
            "create result field subject"
        );
        assert_eq!(
            result["body"],
            serde_json::to_value(&added.body).unwrap(),
            "get result field body"
        );

        assert_eq!(added.body, payload.body, "create result field body");

        assert_eq!(result["_permission"], "owner");

        let ids = serde_json::json!([]);

        assert_eq!(result["comment_ids"], ids, "field comment_ids");

        assert_eq!(
            result["reactions"],
            serde_json::json!([]),
            "field reactions"
        );

        assert_eq!(result["poll"], serde_json::json!(null), "field poll");

        assert_eq!(result["images"], serde_json::json!([]), "field images");

        // Check that we don't return any fields which are supposed to be omitted.

        let result = user
            .client
            .get(&format!("posts/{}", added_objects[1].1.id))
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();

        let (payload, added) = &added_objects[1];
        assert_eq!(
            result["id"],
            serde_json::to_value(&added.id).unwrap(),
            "get result field id"
        );
        assert_eq!(
            result["organization_id"],
            serde_json::to_value(&added.organization_id).unwrap(),
            "get result field organization_id"
        );
        assert_eq!(
            result["updated_at"],
            serde_json::to_value(&added.updated_at).unwrap(),
            "get result field updated_at"
        );
        assert_eq!(
            result["created_at"],
            serde_json::to_value(&added.created_at).unwrap(),
            "get result field created_at"
        );
        assert_eq!(
            result["subject"],
            serde_json::to_value(&added.subject).unwrap(),
            "get result field subject"
        );
        assert_eq!(
            result["body"],
            serde_json::to_value(&added.body).unwrap(),
            "get result field body"
        );
        assert_eq!(result["_permission"], "write");

        // Check that we don't return any fields which are supposed to be omitted.

        let response = no_roles_user
            .client
            .get(&format!("posts/{}", added_objects[1].1.id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

        // TODO test get of child comment object

        // TODO test get of child reaction object

        // TODO test get of child poll object

        // TODO test get of child post_image object
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

        let update_payload = make_update_payload(20);
        admin_user
            .client
            .put(&format!("posts/{}", added_objects[1].1.id))
            .json(&update_payload)
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap();

        let updated: serde_json::Value = admin_user
            .client
            .get(&format!("posts/{}", added_objects[1].1.id))
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            updated["subject"],
            serde_json::to_value(&update_payload.subject).unwrap(),
            "field subject"
        );
        assert_eq!(
            updated["body"],
            serde_json::to_value(&update_payload.body).unwrap(),
            "field body"
        );
        assert_eq!(updated["_permission"], "owner");

        // TODO Test that owner can not write fields which are not writable by anyone.
        // TODO Test that user can not update fields which are writable by owner but not user

        // Make sure that no other objects were updated
        let non_updated: serde_json::Value = admin_user
            .client
            .get(&format!("posts/{}", added_objects[0].1.id))
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            non_updated["id"],
            serde_json::to_value(&added_objects[0].1.id).unwrap(),
            "field id"
        );
        assert_eq!(
            non_updated["organization_id"],
            serde_json::to_value(&added_objects[0].1.organization_id).unwrap(),
            "field organization_id"
        );
        assert_eq!(
            non_updated["updated_at"],
            serde_json::to_value(&added_objects[0].1.updated_at).unwrap(),
            "field updated_at"
        );
        assert_eq!(
            non_updated["created_at"],
            serde_json::to_value(&added_objects[0].1.created_at).unwrap(),
            "field created_at"
        );
        assert_eq!(
            non_updated["subject"],
            serde_json::to_value(&added_objects[0].1.subject).unwrap(),
            "field subject"
        );
        assert_eq!(
            non_updated["body"],
            serde_json::to_value(&added_objects[0].1.body).unwrap(),
            "field body"
        );
        assert_eq!(non_updated["_permission"], "owner");

        let response = no_roles_user
            .client
            .put(&format!("posts/{}", added_objects[1].1.id))
            .json(&update_payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

        // TODO test update endpoint of child comment object

        // TODO test update endpoint of child reaction object

        // TODO test update endpoint of child poll object

        // TODO test update endpoint of child post_image object
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
            .post("posts")
            .json(&create_payload)
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(
            created_result["subject"],
            serde_json::to_value(&create_payload.subject).unwrap(),
            "field subject from create response"
        );
        assert_eq!(
            created_result["body"],
            serde_json::to_value(&create_payload.body).unwrap(),
            "field body from create response"
        );
        assert_eq!(created_result["_permission"], "owner");

        let created_id = created_result["id"].as_str().unwrap();
        let get_result = admin_user
            .client
            .get(&format!("posts/{}", created_id))
            .send()
            .await
            .unwrap()
            .log_error()
            .await
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
            get_result["subject"], created_result["subject"],
            "field subject from get response"
        );
        assert_eq!(
            get_result["body"], created_result["body"],
            "field body from get response"
        );
        assert_eq!(get_result["_permission"], "owner");

        let response = no_roles_user
            .client
            .post("posts")
            .json(&create_payload)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

        // TODO test create endpoint of child comment object

        // TODO test create endpoint of child reaction object

        // TODO test create endpoint of child poll object

        // TODO test create endpoint of child post_image object
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
            .delete(&format!("posts/{}", added_objects[1].1.id))
            .send()
            .await
            .unwrap()
            .log_error()
            .await
            .unwrap();

        let response = admin_user
            .client
            .get(&format!("posts/{}", added_objects[1].1.id))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        // Delete should not happen without permissions
        let response = no_roles_user
            .client
            .delete(&format!("posts/{}", added_objects[0].1.id))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

        // Make sure other objects still exist
        let response = admin_user
            .client
            .get(&format!("posts/{}", added_objects[0].1.id))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        // TODO test delete endpoint of child comment object

        // TODO test delete endpoint of child reaction object

        // TODO test delete endpoint of child poll object

        // TODO test delete endpoint of child post_image object
    }
}
