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

use self::json_schema::SchemaErrors;

pub mod json_schema;

#[derive(Debug)]
pub enum FormOrJsonRejection {
    Validation(SchemaErrors),
    Json(JsonRejection),
    Form(FormRejection),
    Multipart(MultipartRejection),
    MultipartField(MultipartError),
    HtmlForm(serde_html_form::de::Error),
    Serde(serde_path_to_error::Error<serde_json::Error>),
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
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            // Try JSON if there is no content-type, to accomodate curl and similar
            .unwrap_or("application/json");

        let (mut value, coerce_arrays) = if content_type.starts_with("application/json") {
            let value = Json::<serde_json::Value>::from_request(req, _state)
                .await
                .map(|json| FormOrJson(json.0))
                .map_err(FormOrJsonRejection::Json)?
                .0;
            (value, false)
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            let value = Form::<serde_json::Value>::from_request(req, _state)
                .await
                .map_err(FormOrJsonRejection::Form)?
                .0;
            (value, true)
        } else if content_type.starts_with("multipart/form-data") {
            let value = parse_multipart(req).await?;
            (value, true)
        } else {
            return Err(FormOrJsonRejection::UnknownContentType);
        };

        json_schema::validate::<T>(&mut value, coerce_arrays)
            .map_err(FormOrJsonRejection::Validation)?;

        serde_path_to_error::deserialize(value)
            .map(FormOrJson)
            .map_err(FormOrJsonRejection::Serde)
    }
}

async fn parse_multipart(req: Request<Body>) -> Result<serde_json::Value, FormOrJsonRejection> {
    let mut output = json!({});
    let mut multipart = Multipart::from_request(req, &())
        .await
        .map_err(FormOrJsonRejection::Multipart)?;

    while let Some(field) = multipart.next_field().await? {
        let content_type = field.content_type().unwrap_or("text/plain");
        match content_type {
            "application/x-www-form-urlencoded" => {
                let data = field.bytes().await?;
                let json_value: serde_json::value::Map<String, serde_json::Value> =
                    serde_html_form::from_bytes(&data)?;
                for (key, value) in json_value {
                    output[key] = value;
                }
            }
            _ => {
                let name = field.name().map(|s| s.to_string());
                if let Some(name) = name {
                    let filename = field.file_name().map(|s| s.to_string());
                    let data = field.bytes().await?;

                    let file = json!({
                        "filename": filename,
                        "data": Vec::from(data)
                    });

                    match output.get_mut(&name) {
                        Some(serde_json::Value::Array(a)) => {
                            a.push(file);
                        }
                        Some(v) => {
                            let old = v.take();
                            *v = json!([old, file]);
                        }
                        None => {
                            output[name] = file;
                        }
                    }
                }
            }
        }
    }

    Ok(output)
}
