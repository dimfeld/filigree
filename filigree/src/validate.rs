use std::fmt::Debug;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{rejection::JsonRejection, FromRequest, Request},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::{Form, FormRejection};
use hyper::StatusCode;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use self::json_schema::SchemaErrors;

pub mod json_schema;

#[derive(Debug)]
pub enum FormOrJsonRejection {
    Validation(SchemaErrors),
    Json(JsonRejection),
    Form(FormRejection),
    Serde(serde_path_to_error::Error<serde_json::Error>),
    UnknownContentType,
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
            FormOrJsonRejection::Serde(inner) => {
                // TODO common format between this and Validation
                (StatusCode::BAD_REQUEST, inner.to_string()).into_response()
            }
            FormOrJsonRejection::UnknownContentType => {
                (StatusCode::BAD_REQUEST, "Unknown content type").into_response()
            }
        }
    }
}

pub struct FormOrJson<T>(pub T)
where
    T: Debug + JsonSchema + DeserializeOwned;

#[async_trait]
impl<T, S> FromRequest<S> for FormOrJson<T>
where
    T: Debug + JsonSchema + DeserializeOwned,
    S: Sync + Send,
{
    type Rejection = FormOrJsonRejection;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            // Try JSON if there is no content-type, to accomodate curl and similar
            .unwrap_or("application/json");

        let value = if content_type.starts_with("application/json") {
            Json::<serde_json::Value>::from_request(req, _state)
                .await
                .map(|json| FormOrJson(json.0))
                .map_err(FormOrJsonRejection::Json)?
                .0
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            Form::<serde_json::Value>::from_request(req, _state)
                .await
                .map_err(FormOrJsonRejection::Form)?
                .0
        } else if content_type.starts_with("multipart/form-data") {
            todo!("multipart/form-data");
        } else {
            return Err(FormOrJsonRejection::UnknownContentType);
        };

        json_schema::validate::<T>(&value).map_err(FormOrJsonRejection::Validation)?;

        serde_path_to_error::deserialize(value)
            .map(FormOrJson)
            .map_err(FormOrJsonRejection::Serde)
    }
}
