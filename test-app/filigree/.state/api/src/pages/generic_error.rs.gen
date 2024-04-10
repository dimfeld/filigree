use axum::response::{IntoResponse, Response};
use filigree::errors::HttpError;
use maud::html;

use crate::Error;

pub fn generic_error_page(err: &Error) -> Response {
    let body = html! {};

    (err.status_code(), body).into_response()
}
