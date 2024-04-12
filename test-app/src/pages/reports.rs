#![allow(unused_imports)]
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::extract::ValidatedForm;
use maud::{html, Markup};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::{error::HtmlError, layout::root_layout_page},
    server::ServerState,
    Error,
};

pub mod _id;

#[derive(serde::Deserialize, serde::Serialize, Debug, JsonSchema)]
pub struct FavoriteActionPayload {
    pub new_state: bool,
}

fn favorite_action_fragment() -> Markup {
    html! {}
}

async fn favorite_action(
    State(state): State<ServerState>,
    auth: Authed,
    Path(id): Path<String>,
    form: ValidatedForm<FavoriteActionPayload>,
) -> Result<impl IntoResponse, Error> {
    let body = favorite_action_fragment();

    Ok(body)
}

#[derive(serde::Deserialize, serde::Serialize, Debug, JsonSchema)]
pub struct ReportsPayload {
    pub body: String,
    pub subject: String,
}

fn reports_form_fragment() -> Markup {
    html! {}
}

async fn reports_form(
    State(state): State<ServerState>,
    auth: Authed,
    form: ValidatedForm<ReportsPayload>,
) -> Result<impl IntoResponse, HtmlError> {
    let body = reports_form_fragment();

    Ok(body)
}

async fn reports_page(
    State(state): State<ServerState>,
    auth: Option<Authed>,
) -> Result<impl IntoResponse, HtmlError> {
    let body = html! {};

    Ok(root_layout_page(auth.as_ref(), "title", body))
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/reports", routing::get(reports_page))
        .route(
            "/reports",
            routing::post(reports_form)
                .route_layer(has_any_permission(vec!["Report:write", "org_admin"])),
        )
        .route(
            "/reports/_action/favorite/:id",
            routing::post(favorite_action),
        )
        .merge(_id::create_routes())
}
