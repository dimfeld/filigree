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

mod error;
mod forgot;
mod generic_error;
mod layout;
mod login;
mod logout;
mod not_found;
mod reports;
mod reset;

pub use generic_error::*;
use layout::*;
pub use not_found::*;

async fn home(auth: Option<Authed>) -> impl IntoResponse {
    root_layout_page(auth.as_ref(), "Home", html! { h1 { "Home" } })
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new()
        .route("/", routing::get(home))
        .merge(login::create_routes())
        .merge(logout::create_routes())
        .merge(forgot::create_routes())
        .merge(reset::create_routes())
        .merge(reports::create_routes())
        .fallback(|| async { not_found_page() })
}
