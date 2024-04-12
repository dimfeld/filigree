use crate::server::ServerState;

pub mod public;

pub fn create_routes() -> axum::Router<ServerState> {
    axum::Router::new().merge(public::create_routes())
}
