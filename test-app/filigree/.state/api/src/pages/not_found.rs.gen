use axum::response::{IntoResponse, Response};
use filigree::html_elements;
use http::StatusCode;
use hypertext::rsx;

use super::root_layout_page;

/// Render the not found page. This function is called from the router when no other routes match.
pub async fn not_found_fallback() -> Response {
    not_found_page()
}

/// Render the not found page from any context.
pub fn not_found_page() -> Response {
    let body = rsx! {
        <p>"Couldn't find this page"</p>
    };

    (
        StatusCode::NOT_FOUND,
        root_layout_page(None, "not found", body),
    )
        .into_response()
}
