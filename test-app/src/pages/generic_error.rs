use axum::response::{IntoResponse, Response};
use maud::html;

use crate::Error;

pub fn generic_error_page(err: &Error) -> Response {
    let body = html! {};

    body.into_response()
}
