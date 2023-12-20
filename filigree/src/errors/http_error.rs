use std::borrow::Cow;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::{event, Level};

/// An error that can be returned from an HTTP endpoint
pub trait HttpError: ToString + std::fmt::Debug {
    /// The status code that the error should return.
    fn status_code(&self) -> StatusCode;
    /// An error code that may provide additional information to clients on how to behave in
    /// response to the error.
    fn error_kind(&self) -> &'static str;

    /// The status code and data for this error. Most implementors of this trait will not
    /// need to override the default implementation.
    fn response_tuple(&self) -> (StatusCode, ErrorResponseData) {
        (
            self.status_code(),
            ErrorResponseData::new(
                self.error_kind(),
                self.to_string(),
                Some(format!("{self:?}")),
            ),
        )
    }

    /// Convert the error into a [Response]. Most implementors of this trait will not
    /// need to override the default implementation.
    fn to_response(&self) -> Response {
        let (code, json) = self.response_tuple();
        (code, Json(json)).into_response()
    }
}

/// A body to be returned in an error response
#[derive(Debug, Serialize)]
pub struct ErrorResponseData {
    error: ErrorDetails,
}

/// An error code and additional details.
#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    kind: Cow<'static, str>,
    message: Cow<'static, str>,
    details: Option<String>,
}

impl ErrorResponseData {
    /// Create a new [ErrorResponseData] with the given error code and message.
    pub fn new(
        kind: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
        details: Option<String>,
    ) -> ErrorResponseData {
        let ret = ErrorResponseData {
            error: ErrorDetails {
                kind: kind.into(),
                message: message.into(),
                details,
            },
        };

        event!(Level::ERROR, kind=%ret.error.kind, message=%ret.error.message, details=?ret.error.details);

        ret
    }
}
