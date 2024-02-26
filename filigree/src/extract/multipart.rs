use std::fmt::Debug;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Request},
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use super::Rejection;
use crate::requests::{file::FileUpload, multipart::parse_multipart};

/// Extract a multipart form submission and perform JSON schema validation.
/// The `data` field contains all the non-file submissions, and the uploaded files
/// are placed in the `files` field.
///
/// This extractor buffers all the uploaded files. If you prefer to process them as streams,
/// use [MultipartProcessor] instead..
pub struct Multipart<T>
where
    T: DeserializeOwned + JsonSchema + Debug + Send + Sync + 'static,
{
    /// The non-file data
    pub data: T,
    /// The files attached to the request.
    pub files: Vec<FileUpload>,
}

#[async_trait]
impl<S, T> FromRequest<S> for Multipart<T>
where
    S: Send + Sync,
    T: DeserializeOwned + JsonSchema + Debug + Send + Sync + 'static,
{
    type Rejection = Rejection;

    async fn from_request(req: Request<Body>, _: &S) -> Result<Self, Self::Rejection> {
        let (data, files) = parse_multipart(req).await?;

        let data = crate::requests::json_schema::validate::<T>(data, true)
            .map_err(Rejection::Validation)?;

        let data = serde_path_to_error::deserialize(data).map_err(Rejection::Serde)?;

        Ok(Self { data, files })
    }
}
