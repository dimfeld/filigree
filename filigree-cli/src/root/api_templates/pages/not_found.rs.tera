use axum::response::{IntoResponse, Response};
use http::StatusCode;
use maud::html;

use crate::Error;

pub fn not_found_page() -> Response {
    let body = html! {};

    (StatusCode::NOT_FOUND, body).into_response()
}
