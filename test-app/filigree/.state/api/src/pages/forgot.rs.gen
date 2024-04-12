use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::{extract::FormOrJson, html_elements};
use hypertext::{rsx, Renderable};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::{error::HtmlError, layout::root_layout_page},
    server::ServerState,
};

#[derive(serde::Deserialize, Debug, JsonSchema)]
pub struct ForgotPayload {
    email: String,
}

async fn forgot_form(
    State(state): State<ServerState>,
    FormOrJson(payload): FormOrJson<ForgotPayload>,
) -> Result<impl IntoResponse, HtmlError> {
    Ok(rsx! {}.render())
}

async fn forgot_page(State(state): State<ServerState>) -> impl IntoResponse {
    root_layout_page(None, "Forgot", rsx! { <h1>Forgot</h1> })
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/forgot", routing::get(forgot_page))
        .route("/forgot", routing::post(forgot_form))
}
