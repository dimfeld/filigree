use std::{borrow::Cow, fmt::Debug, ops::Deref};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use error_stack::Report;
use serde::Serialize;
use tracing::{event, Level};

/// An error that can be returned from an HTTP endpoint
pub trait HttpError: ToString + std::fmt::Debug {
    type Detail: Serialize + Debug + Send + Sync + 'static;

    /// The status code that the error should return.
    fn status_code(&self) -> StatusCode;
    /// An error code that may provide additional information to clients on how to behave in
    /// response to the error.
    fn error_kind(&self) -> &'static str;

    /// Extra detail about this error
    fn error_detail(&self) -> Self::Detail;

    /// The status code and data for this error. Most implementors of this trait will not
    /// need to override the default implementation.
    fn response_tuple(&self) -> (StatusCode, ErrorResponseData<Self::Detail>) {
        let detail = self.error_detail();

        let detail = (!detail.is_empty()).then_some(detail);

        (
            self.status_code(),
            ErrorResponseData::new(self.error_kind(), self.to_string(), self.error_detail()),
        )
    }

    /// Convert the error into a [Response]. Most implementors of this trait will not
    /// need to override the default implementation.
    fn to_response(&self) -> Response {
        let (code, json) = self.response_tuple();
        (code, Json(json)).into_response()
    }
}

impl<T> HttpError for error_stack::Report<T>
where
    T: HttpError + Send + Sync + 'static,
{
    type Detail = String;

    fn status_code(&self) -> StatusCode {
        self.current_context().status_code()
    }

    fn error_kind(&self) -> &'static str {
        self.current_context().error_kind()
    }

    /// Send the entire report detail as the detail
    fn error_detail(&self) -> String {
        format!("{self:?}")
    }
}

/// A body to be returned in an error response
#[derive(Debug, Serialize)]
pub struct ErrorResponseData<T: Debug + Serialize> {
    error: ErrorDetails<T>,
}

/// An error code and additional details.
#[derive(Debug, Serialize)]
pub struct ErrorDetails<T: Debug + Serialize> {
    kind: Cow<'static, str>,
    message: Cow<'static, str>,
    details: T,
}

impl<T: Debug + Serialize> ErrorResponseData<T> {
    /// Create a new [ErrorResponseData] with the given error code and message.
    pub fn new(
        kind: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
        details: T,
    ) -> ErrorResponseData<T> {
        let ret = ErrorResponseData {
            error: ErrorDetails {
                kind: kind.into(),
                message: message.into(),
                details: details.into(),
            },
        };

        event!(Level::ERROR, kind=%ret.error.kind, message=%ret.error.message, details=?ret.error.details);

        ret
    }
}

/// Wraps an error_stack::Report and implements IntoResponse, allowing easy return of a Report<T>
/// from an Axum endpoint.
pub struct WrapReport<T: HttpError + Sync + Send + 'static>(error_stack::Report<T>);

impl<T: HttpError + Sync + Send + 'static> IntoResponse for WrapReport<T> {
    fn into_response(self) -> Response {
        self.0.to_response()
    }
}

impl<T: HttpError + Sync + Send + 'static> From<Report<T>> for WrapReport<T> {
    fn from(value: Report<T>) -> Self {
        WrapReport(value)
    }
}

impl<T: HttpError + std::error::Error + Sync + Send + 'static> From<T> for WrapReport<T> {
    fn from(value: T) -> Self {
        WrapReport(Report::from(value))
    }
}

impl<T: HttpError + Sync + Send + 'static> Deref for WrapReport<T> {
    type Target = error_stack::Report<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
