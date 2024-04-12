#![allow(unused_imports)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::extract::ValidatedForm;
use maud::html;
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::{error::HtmlError, layout::root_layout_page},
    server::ServerState,
    Error,
};

pub mod edit;

pub mod stats;

pub mod views;

async fn reports_page(
    State(state): State<ServerState>,
    auth: Option<Authed>,
    Path(id): Path<crate::models::report::ReportId>,
) -> Result<impl IntoResponse, HtmlError> {
    let body = html! {};

    Ok(root_layout_page(auth.as_ref(), "title", body))
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/reports/:id", routing::get(reports_page))
        .merge(edit::create_routes())
        .merge(stats::create_routes())
        .merge(views::create_routes())
}
