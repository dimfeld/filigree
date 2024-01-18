pub mod organization;
pub mod report;
pub mod role;
pub mod user;

use axum::Router;

use crate::server::ServerState;

pub fn create_routes() -> Router<ServerState> {
    Router::new()
        .merge(user::endpoints::create_routes())
        .merge(role::endpoints::create_routes())
        .merge(report::endpoints::create_routes())
}
