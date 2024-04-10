use axum::response::{IntoResponse, Response};
use http::StatusCode;
use maud::html;

/// Render the not found page. This function is called from the router when no other routes match.
pub async fn not_found_fallback() -> Response {
    not_found_page()
}

/// Render the not found page from any context.
pub fn not_found_page() -> Response {
    let body = html! {};

    (StatusCode::NOT_FOUND, body).into_response()
}
