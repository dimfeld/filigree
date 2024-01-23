use std::fmt::Debug;

use axum::{extract::rejection::JsonRejection, response::IntoResponse};
use axum_extra::extract::{
    multipart::{MultipartError, MultipartRejection},
    FormRejection,
};
use hyper::StatusCode;

use crate::requests::json_schema::SchemaErrors;

mod form_or_json;
mod multipart;

pub use form_or_json::*;
pub use multipart::*;

#[derive(Debug)]
pub enum Rejection {
    Validation(SchemaErrors),
    ReadBody(axum::Error),
    Json(JsonRejection),
    Form(FormRejection),
    Multipart(MultipartRejection),
    MultipartField(MultipartError),
    Serde(serde_path_to_error::Error<serde_json::Error>),
    MissingData,
    UnknownContentType,
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

impl IntoResponse for Rejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            Rejection::Validation(inner) => {
                // Put together a proper format here
                todo!()
            }
            Rejection::ReadBody(inner) => {
                todo!()
            }
            Rejection::Form(inner) => inner.into_response(),
            Rejection::Json(inner) => inner.into_response(),
            Rejection::Multipart(inner) => inner.into_response(),
            Rejection::MultipartField(inner) => {
                todo!()
            }
            Rejection::Serde(inner) => {
                // TODO common format between this and Validation
                (StatusCode::BAD_REQUEST, inner.to_string()).into_response()
            }
            Rejection::UnknownContentType => {
                (StatusCode::BAD_REQUEST, "Unknown content type").into_response()
            }
            Rejection::MissingData => todo!(),
        }
    }
}
