use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use maud::{html, Markup, DOCTYPE};

use crate::{
    auth::{has_any_permission, Authed},
    server::ServerState,
    Error,
};

mod forgot;
mod layout;
mod login;
mod logout;
mod reports;
mod reset;

use layout::*;

async fn home(auth: Option<Authed>) -> impl IntoResponse {
    root_layout(auth.as_ref(), "Home", html! { h1 { "Home" } })
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/", routing::get(home))
        .merge(login::create_routes())
        .merge(logout::create_routes())
        .merge(forgot::create_routes())
        .merge(reset::create_routes())
        .merge(reports::create_routes())
}
