use axum::response::{IntoResponse, Response};
use maud::html;

use crate::Error;

pub fn not_found_page() -> Response {
    let body = html! {};

    body.into_response()
}
