#![allow(unused_imports)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::extract::ValidatedForm;
use hypertext::{maud, Renderable};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::{error::HtmlError, layout::root_layout_page},
    server::ServerState,
    Error,
};

async fn public_page(
    State(state): State<ServerState>,
    auth: Option<Authed>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, HtmlError> {
    let body = maud! {};

    Ok(root_layout_page(auth.as_ref(), "title", body))
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new().route("/reports/:id/views/public", routing::get(public_page))
}
