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
        .route(
            "/user",
            routing::post(create).route_layer(has_permission(OWNER_PERMISSION)),
        )
        .route("/user/:id", routing::put(update))
        .route("/user/:id", routing::delete(delete))
}
