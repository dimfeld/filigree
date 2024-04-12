use axum::response::{IntoResponse, Response};
use filigree::{errors::HttpError, html_elements};
use http::StatusCode;
use hypertext::maud;

use super::root_layout_page;
use crate::Error;

pub fn generic_error_page(err: &Error) -> Response {
    let body = maud! {
        p { "Sorry, we encountered an unexpected error" }
    };

    (StatusCode::NOT_FOUND, root_layout_page(None, "Error", body)).into_response()
}
