use std::any::Any;

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
};
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

use super::http_error::ErrorResponseData;

/// A middleware that handles panics in the application
fn handle_panic(production: bool, err: Box<dyn Any + Send + 'static>) -> Response {
    let body = if production {
        ErrorResponseData::new("internal_server_error", "Server error", None)
    } else {
        let details = if let Some(s) = err.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = err.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic message".to_string()
        };

        ErrorResponseData::new("panic", details, None)
    };

    let body = serde_json::to_string(&body).unwrap_or_default();

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// A middleware that handles panics in the application and returns the error formatted as JSON
/// If `production` is true, this will return a generic error instead of the actual error details.
pub fn panic_handler(production: bool) -> CatchPanicLayer<impl ResponseForPanic> {
    CatchPanicLayer::custom(move |err| handle_panic(production, err))
}
