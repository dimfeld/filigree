#[allow(unused_imports)]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use axum_extra::extract::{Form, Query};
use filigree::extract::ValidatedForm;
use maud::{html, Markup};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::{auth::WebAuthed, error::HtmlError},
    server::ServerState,
    Error,
};

mod auth;
mod error;
mod forgot;
mod generic_error;
pub mod layout;
mod login;
mod logout;
pub mod not_found;
mod reports;
mod reset;

pub use generic_error::*;
use layout::*;
pub use not_found::*;

fn count_action_fragment() -> Markup {
    html! {}
}

async fn count_action(
    State(state): State<ServerState>,
    auth: Option<Authed>,
) -> Result<impl IntoResponse, Error> {
    let body = count_action_fragment();

    Ok(body)
}

async fn home_page(
    State(state): State<ServerState>,
    auth: Option<WebAuthed>,
) -> Result<impl IntoResponse, HtmlError> {
    let body = html! {};

    Ok(root_layout_page(auth.as_ref(), "title", body))
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/", routing::get(home_page))
        .route("/_action/count", routing::post(count_action))
        .merge(login::create_routes())
        .merge(logout::create_routes())
        .merge(forgot::create_routes())
        .merge(reset::create_routes())
        .merge(reports::create_routes())
}
