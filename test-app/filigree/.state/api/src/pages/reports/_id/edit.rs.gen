#![allow(unused_imports)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::extract::FormOrJson;
use maud::{html, Markup};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::layout::root_layout_page,
    server::ServerState,
    Error,
};

async fn edit_page(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let body = html! {};

    Ok(root_layout_page(auth.as_ref(), "title", body))
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new().route(
        "/reports/:id/edit",
        routing::get(edit_page).route_layer(has_any_permission(vec!["Report:write", "org_admin"])),
    )
}
