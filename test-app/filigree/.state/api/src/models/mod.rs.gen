pub mod comment;
pub mod organization;
pub mod poll;
pub mod post;
pub mod reaction;
pub mod report;
pub mod report_section;
pub mod role;
pub mod user;

use axum::Router;

use crate::server::ServerState;

pub fn create_routes() -> Router<ServerState> {
    Router::new()
        .merge(post::endpoints::create_routes())
        .merge(report::endpoints::create_routes())
        .merge(role::endpoints::create_routes())
        .merge(user::endpoints::create_routes())
}
