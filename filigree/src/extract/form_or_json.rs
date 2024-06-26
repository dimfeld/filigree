use std::{fmt::Debug, ops::Deref};

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Request},
    Json, RequestExt,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use super::Rejection;
use crate::requests::{
    json_schema::ValidationErrorResponse, multipart::parse_multipart,
    urlencoded::value_from_urlencoded, ContentType,
};

/// Extract a body from either JSON or form submission, and perform JSON schema validation.
#[derive(Debug)]
pub struct FormOrJson<T>(pub T)
where
    T: Debug + JsonSchema + DeserializeOwned;

impl<T> Deref for FormOrJson<T>
where
    T: Debug + JsonSchema + DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for FormOrJson<T>
where
    T: Debug + JsonSchema + DeserializeOwned + 'static,
    S: Sync + Send,
{
    type Rejection = Rejection;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let (data, _) = form_or_json_from_request(req, _state).await?;
        Ok(FormOrJson(data))
    }
}

/// Result of extracting and validating a form body
pub type FormOrJsonResult<T> = Result<T, Rejection>;

/// The result of a form validation, unpacked for use in server-side rendering
#[derive(Debug)]
pub struct ValidatedForm<T: Debug + JsonSchema + DeserializeOwned + 'static> {
    /// The data, if it validated properly
    pub data: Option<T>,
    /// The form that was submitted, if the validation failed
    pub form: serde_json::Value,
    /// The errors encountered
    pub errors: ValidationErrorResponse,
}

impl<T: Debug + JsonSchema + DeserializeOwned + 'static> Default for ValidatedForm<T> {
    fn default() -> Self {
        Self {
            data: None,
            form: serde_json::Value::Object(serde_json::Map::new()),
            errors: ValidationErrorResponse::default(),
        }
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: Debug + JsonSchema + DeserializeOwned + 'static,
    S: Sync + Send,
{
    type Rejection = Rejection;

    async fn from_request(req: Request<Body>, _state: &S) -> Result<Self, Self::Rejection> {
        let value = form_or_json_from_request(req, _state).await;

        match value {
            Ok((data, form)) => Ok(ValidatedForm {
                data: Some(data),
                form,
                errors: ValidationErrorResponse::default(),
            }),
            Err(Rejection::Validation((form, errors))) => Ok(ValidatedForm {
                data: None,
                form,
                errors: ValidationErrorResponse::from(errors),
            }),
            Err(e) => Err(e),
        }
    }
}

async fn form_or_json_from_request<T, S>(
    req: Request<Body>,
    _state: &S,
) -> Result<(T, serde_json::Value), Rejection>
where
    T: Debug + JsonSchema + DeserializeOwned + 'static,
    S: Sync + Send,
{
    let content_type = ContentType::new(
        req.headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            // Try JSON if there is no content-type, to accomodate lazy uses of curl and similar
            .unwrap_or("application/json"),
    );

    let (value, coerce_arrays) = if content_type.is_json() {
        let value = Json::<serde_json::Value>::from_request(req, _state)
            .await
            .map(|json| FormOrJson(json.0))
            .map_err(Rejection::Json)?
            .0;
        (value, false)
    } else if content_type.is_form() {
        let bytes = axum::body::to_bytes(req.into_limited_body(), usize::MAX)
            .await
            .map_err(Rejection::ReadBody)?;
        let value = value_from_urlencoded(&bytes);
        (value, true)
    } else if content_type.is_multipart() {
        let (value, _) = parse_multipart::<serde_json::Value>(req).await?;
        (value, true)
    } else {
        return Err(Rejection::UnsupportedContentType);
    };

    let value = crate::requests::json_schema::validate::<T>(value, coerce_arrays)
        .map_err(Rejection::Validation)?;

    let data = serde_path_to_error::deserialize(&value).map_err(Rejection::Serde)?;

    Ok((data, value))
}

#[cfg(test)]
mod test {
    use schemars::JsonSchema;
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize, Debug, PartialEq, Eq, JsonSchema)]
    #[serde(untagged)]
    enum NumOrBool {
        Num(i32),
        Bool(bool),
    }

    #[derive(Deserialize, JsonSchema, Debug, PartialEq, Eq)]
    struct Data {
        s: String,
        s_vec1: Vec<String>,
        s_vec2: Vec<String>,
        i: i32,
        i_vec1: Vec<i32>,
        i_vec2: Vec<i32>,
        nob_n: NumOrBool,
        nob_b: NumOrBool,
        nob_vec: Vec<NumOrBool>,
        b: bool,
        b_omitted: bool,
        ob: Option<bool>,
        b_vec1: Vec<bool>,
        b_vec2: Vec<bool>,
    }

    #[tokio::test]
    async fn extract_from_json() {}

    #[tokio::test]
    async fn extract_from_form() {
        let body = "s=a&s_vec1=a&s_vec2=a&s_vec2=b&i=1&i_vec1=1&i_vec2=1&i_vec2=2&nob_n=1&nob_b=on&nob_vec=1&nob_vec=on&b=true&b_vec1=true&b_vec2=on&b_vec2=false";
        let data = hyper::Request::builder()
            .header("content-type", "application/x-www-form-urlencoded")
            .header("content-length", body.len())
            .body(axum::body::Body::from(body))
            .unwrap();

        let data = FormOrJson::<Data>::from_request(data, &()).await.unwrap();

        assert_eq!(
            data.0,
            Data {
                s: "a".to_string(),
                s_vec1: vec!["a".to_string()],
                s_vec2: vec!["a".to_string(), "b".to_string()],
                i: 1,
                i_vec1: vec![1],
                i_vec2: vec![1, 2],
                nob_n: NumOrBool::Num(1),
                nob_b: NumOrBool::Bool(true),
                nob_vec: vec![NumOrBool::Num(1), NumOrBool::Bool(true)],
                b: true,
                b_omitted: false,
                ob: None,
                b_vec1: vec![true],
                b_vec2: vec![true, false],
            }
        )
    }
}
