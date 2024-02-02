#![allow(unused_imports, dead_code)]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use axum_extra::extract::Query;
use axum_jsonschema::Json;

use super::{
    queries, types::*, OrganizationId, CREATE_PERMISSION, OWNER_PERMISSION, READ_PERMISSION,
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
    Path(id): Path<OrganizationId>,
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
    Json(payload): Json<OrganizationCreatePayload>,
) -> Result<impl IntoResponse, Error> {
    let result = queries::create(&state.db, &auth, &payload).await?;

    Ok((StatusCode::CREATED, Json(result)))
}

async fn update(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<OrganizationId>,
    Json(payload): Json<OrganizationUpdatePayload>,
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
    Path(id): Path<OrganizationId>,
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
}

#[cfg(test)]
mod test {
    use filigree::testing::ResponseExt;
    use futures::{StreamExt, TryStreamExt};
    use tracing::{event, Level};

    use super::*;
    use crate::tests::{start_app, BootstrappedData};

    fn make_create_payload(i: usize) -> OrganizationCreatePayload {
        OrganizationCreatePayload {
            name: format!("Test object {i}"),
            owner: (i > 1).then(|| <crate::models::user::UserId as Default>::default()),
            default_role: (i > 1).then(|| <crate::models::role::RoleId as Default>::default()),
        }
    }

    async fn setup_test_objects(
        db: &sqlx::PgPool,
        organization_id: OrganizationId,
        count: usize,
    ) -> Vec<Organization> {
        futures::stream::iter(1..=count)
            .map(Ok)
            .and_then(|i| async move {
                let id = OrganizationId::new();
                event!(Level::INFO, %id, "Creating test object {}", i);
                super::queries::create_raw(db, id, organization_id, &make_create_payload(i)).await
            })
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
    }
}
