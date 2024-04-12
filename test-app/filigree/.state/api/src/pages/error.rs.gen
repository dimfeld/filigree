use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Redirect, Response},
};
use filigree::errors::HttpError;
use http::{StatusCode, Uri};

use super::auth::make_login_link;
use crate::Error;

pub struct HtmlError(pub Error);

impl From<Error> for HtmlError {
    fn from(value: Error) -> Self {
        HtmlError(value)
    }
}

impl From<error_stack::Report<Error>> for HtmlError {
    fn from(value: error_stack::Report<Error>) -> Self {
        HtmlError(Error::WrapReport(value))
    }
}

impl IntoResponse for HtmlError {
    fn into_response(self) -> Response {
        match self.0.status_code() {
            StatusCode::NOT_FOUND => super::not_found::not_found_page(),
            StatusCode::UNAUTHORIZED => unauthenticated_error(&self.0),
            _ => super::generic_error::generic_error_page(&self.0),
        }
    }
}

fn unauthenticated_error(err: &Error) -> Response {
    axum::response::Redirect::to(&make_login_link(None)).into_response()
}

pub fn handle_page_error(uri: Uri, err: Error) -> Response {
    match err.status_code() {
        StatusCode::NOT_FOUND => super::not_found::not_found_page(),
        StatusCode::UNAUTHORIZED => Redirect::to(&make_login_link(Some(&uri))).into_response(),
        _ => super::generic_error::generic_error_page(&err),
    }
}
