use std::fmt::Debug;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{rejection::JsonRejection, FromRequest, Request},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::{
    multipart::{Multipart, MultipartError, MultipartRejection},
    Form, FormRejection,
};
use hyper::StatusCode;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::{file::FileUpload, json_schema::SchemaErrors, ContentType};

#[derive(Debug)]
pub enum FormOrJsonRejection {
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

impl From<MultipartError> for FormOrJsonRejection {
    fn from(err: MultipartError) -> Self {
        FormOrJsonRejection::MultipartField(err)
    }
}

impl From<serde_html_form::de::Error> for FormOrJsonRejection {
    fn from(err: serde_html_form::de::Error) -> Self {
        FormOrJsonRejection::HtmlForm(err)
    }
}

impl From<serde_path_to_error::Error<serde_json::Error>> for FormOrJsonRejection {
    fn from(err: serde_path_to_error::Error<serde_json::Error>) -> Self {
        FormOrJsonRejection::Serde(err)
    }
}

impl IntoResponse for FormOrJsonRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            FormOrJsonRejection::Validation(inner) => {
                // Put together a proper format here
                todo!()
            }
            FormOrJsonRejection::Form(inner) => inner.into_response(),
            FormOrJsonRejection::Json(inner) => inner.into_response(),
            FormOrJsonRejection::Multipart(inner) => inner.into_response(),
            FormOrJsonRejection::MultipartField(inner) => {
                todo!()
            }
            FormOrJsonRejection::HtmlForm(inner) => {
                todo!()
            }
            FormOrJsonRejection::Serde(inner) => {
                // TODO common format between this and Validation
                (StatusCode::BAD_REQUEST, inner.to_string()).into_response()
            }
            FormOrJsonRejection::UnknownContentType => {
                (StatusCode::BAD_REQUEST, "Unknown content type").into_response()
            }
            FormOrJsonRejection::MissingData => todo!(),
        }
    }
}

pub struct FormOrJson<T>(pub T)
where
    T: Debug + JsonSchema + DeserializeOwned;

#[async_trait]
impl<T, S> FromRequest<S> for FormOrJson<T>
where
    T: Debug + JsonSchema + DeserializeOwned + 'static,
    S: Sync + Send,
{
    type Rejection = FormOrJsonRejection;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type = ContentType::new(
            req.headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                // Try JSON if there is no content-type, to accomodate lazy curl and similar
                .unwrap_or("application/json"),
        );

        let (mut value, coerce_arrays) = if content_type.is_json() {
            let value = Json::<serde_json::Value>::from_request(req, _state)
                .await
                .map(|json| FormOrJson(json.0))
                .map_err(FormOrJsonRejection::Json)?
                .0;
            (value, false)
        } else if content_type.is_form() {
            let value = Form::<serde_json::Value>::from_request(req, _state)
                .await
                .map_err(FormOrJsonRejection::Form)?
                .0;
            (value, true)
        } else {
            return Err(FormOrJsonRejection::UnknownContentType);
        };

        super::json_schema::validate::<T>(&mut value, coerce_arrays)
            .map_err(FormOrJsonRejection::Validation)?;

        serde_path_to_error::deserialize(value)
            .map(FormOrJson)
            .map_err(FormOrJsonRejection::Serde)
    }
}
