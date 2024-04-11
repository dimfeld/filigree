//! Maud Utilities

use axum::response::IntoResponse;
use maud::Markup;

/// A wrapper for `Markup` that implements `IntoResponse`
pub struct Html(Markup);

impl IntoResponse for Html {
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.0.into_string()).into_response()
    }
}
