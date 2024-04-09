use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::{
    auth::password::{login_with_password, EmailAndPassword},
    extract::FormOrJson,
};
use maud::{html, Markup, DOCTYPE};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::layout::root_layout_page,
    server::ServerState,
    Error,
};

#[derive(serde::Deserialize, Debug)]
struct RedirectTo {
    redirect_to: Option<String>,
}

async fn login_form(
    State(state): State<ServerState>,
    FormOrJson(payload): FormOrJson<EmailAndPassword>,
    Query(query): Query<RedirectTo>,
) -> impl IntoResponse {
    Ok(html! {})
}

async fn login_page(State(state): State<ServerState>) -> impl IntoResponse {
    root_layout_page(None, "Login", html! { h1 { "Login" } })
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/login", routing::get(login_page))
        .route("/login", routing::post(login_form))
}
