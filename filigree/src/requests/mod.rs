use std::{borrow::Cow, fmt::Display};

use axum::{extract::rejection::JsonRejection, response::IntoResponse};
use axum_extra::extract::{
    multipart::{MultipartError, MultipartRejection},
    FormRejection,
};
use hyper::StatusCode;

use self::json_schema::SchemaErrors;

pub mod file;
pub mod form_or_json;
pub mod json_schema;
pub mod multipart;

#[derive(Debug, Clone)]
pub struct ContentType<'a>(pub Cow<'a, str>);

impl<'a> Display for ContentType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> ContentType<'a> {
    pub fn new(content_type: impl Into<Cow<'a, str>>) -> Self {
        Self(content_type.into())
    }

    pub fn is_json(&self) -> bool {
        self.0.starts_with("application/json")
    }

    pub fn is_form(&self) -> bool {
        self.0.starts_with("application/x-www-form-urlencoded")
    }
}

#[derive(Debug)]
pub enum Rejection {
    Validation(SchemaErrors),
    Json(JsonRejection),
    Form(FormRejection),
    Multipart(MultipartRejection),
    MultipartField(MultipartError),
    HtmlForm(serde_html_form::de::Error),
    Serde(serde_path_to_error::Error<serde_json::Error>),
    MissingData,
    UnknownContentType,
}

impl From<MultipartError> for Rejection {
    fn from(err: MultipartError) -> Self {
        Rejection::MultipartField(err)
    }
}

impl From<serde_html_form::de::Error> for Rejection {
    fn from(err: serde_html_form::de::Error) -> Self {
        Rejection::HtmlForm(err)
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
            Rejection::Form(inner) => inner.into_response(),
            Rejection::Json(inner) => inner.into_response(),
            Rejection::Multipart(inner) => inner.into_response(),
            Rejection::MultipartField(inner) => {
                todo!()
            }
            Rejection::HtmlForm(inner) => {
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
