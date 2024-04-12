#![allow(unused_imports)]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::extract::ValidatedForm;
use hypertext::{maud, Renderable};
use schemars::JsonSchema;

use crate::{
    auth::{has_any_permission, Authed},
    pages::error::HtmlError,
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

async fn count_action(
    State(state): State<ServerState>,
    auth: Option<Authed>,
) -> Result<impl IntoResponse, Error> {
    let body = maud! {}.render();

    Ok(body)
}

async fn home_page(
    State(state): State<ServerState>,
    auth: Option<Authed>,
) -> Result<impl IntoResponse, HtmlError> {
    let body = maud! {};

    Ok(root_layout_page(auth.as_ref(), "Home", body))
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
