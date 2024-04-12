use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use filigree::auth::password::{login_with_password, EmailAndPassword};
use maud::html;

use crate::{
    auth::{has_any_permission, Authed},
    pages::layout::root_layout_page,
    server::ServerState,
    Error,
};

async fn logout_page(State(state): State<ServerState>) -> impl IntoResponse {
    root_layout_page(None, "Logout", html! { h1 { "Logout" } })
}

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new().route("/logout", routing::get(logout_page))
}
