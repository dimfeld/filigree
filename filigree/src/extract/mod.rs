use std::{fmt::Debug, sync::Arc};

use axum::{extract::rejection::JsonRejection, response::IntoResponse, Json};
use axum_extra::extract::{
    multipart::{MultipartError, MultipartRejection},
    FormRejection,
};
use hyper::StatusCode;
use serde::Serialize;

use crate::{
    errors::ErrorResponseData,
    requests::json_schema::{SchemaErrors, ValidationErrorResponse},
};

mod form_or_json;
mod multipart;

pub use form_or_json::*;
pub use multipart::*;

/// Types of errors that the extraction middleware can return
#[derive(Debug)]
pub enum Rejection {
    /// Encountered validation errors on a request payload
    Validation((serde_json::Value, SchemaErrors)),
    /// Failed while reading a request body
    ReadBody(axum::Error),
    /// Failed to deserialize a JSON payload
    Json(JsonRejection),
    /// Failed to deserialize a Form payload
    Form(FormRejection),
    /// Failed to deserialize a Multipart payload
    Multipart(MultipartRejection),
    /// Failed to deserialize a field in a Multipart payload
    MultipartField(MultipartError),
    /// Failed to deserialize a JSON payload
    Serde(serde_path_to_error::Error<serde_json::Error>),
    /// The client passed a content-type header which we don't support
    UnsupportedContentType,
}

impl From<MultipartError> for Rejection {
    fn from(err: MultipartError) -> Self {
        Rejection::MultipartField(err)
    }
}

impl From<serde_path_to_error::Error<serde_json::Error>> for Rejection {
    fn from(err: serde_path_to_error::Error<serde_json::Error>) -> Self {
        Rejection::Serde(err)
    }
}

#[derive(Debug, Serialize)]
struct SerdePathToErrorDetail {
    path: String,
    line: usize,
    column: usize,
    problem: String,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            Rejection::Validation((data, err)) => (
                StatusCode::BAD_REQUEST,
                Json(
                    ErrorResponseData::new(
                        "validation",
                        "Validation Failure",
                        ValidationErrorResponse::from(err),
                    )
                    .with_form(Some(Arc::new(data))),
                ),
            )
                .into_response(),

            Rejection::ReadBody(_) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseData::new(
                    "request_terminated_early",
                    "Request terminated early",
                    (),
                )),
            )
                .into_response(),
            Rejection::Form(inner) => inner.into_response(),
            Rejection::Json(inner) => inner.into_response(),
            Rejection::Multipart(inner) => inner.into_response(),
            Rejection::MultipartField(inner) => inner.into_response(),
            Rejection::Serde(err) => {
                let inner = err.inner();

                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponseData::new(
                        "deserialization_error",
                        "Failed to deserialize request",
                        SerdePathToErrorDetail {
                            path: err.path().to_string(),
                            line: inner.line(),
                            column: inner.column(),
                            problem: inner.to_string(),
                        },
                    )),
                )
                    .into_response()
            }
            Rejection::UnsupportedContentType => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponseData::new(
                    "content_type",
                    "Unsupported content type",
                    (),
                )),
            )
                .into_response(),
        }
    }
}
