use std::fmt::Debug;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Request},
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::{
    file::{FileData, FileUpload},
    urlencoded::value_from_urlencoded,
    ContentType, Rejection,
};

fn coerce_and_push_array(output: &mut serde_json::Value, key: String, value: serde_json::Value) {
    match output.get_mut(&key) {
        Some(serde_json::Value::Array(a)) => {
            a.push(value);
        }
        Some(v) => {
            let old = v.take();
            *v = json!([old, value]);
        }
        None => {
            output[key] = value;
        }
    }
}

/// Parse a multipart form submission into the specified type and a list of files uploaded with it.
pub async fn parse_multipart(
    req: Request<Body>,
) -> Result<(serde_json::Value, Vec<FileUpload>), Rejection> {
    let mut output = json!({});
    let mut files = Vec::new();

    let mut multipart = axum_extra::extract::multipart::Multipart::from_request(req, &())
        .await
        .map_err(Rejection::Multipart)?;

    while let Some(field) = multipart.next_field().await? {
        let content_type = ContentType::new(field.content_type().unwrap_or_default());

        if let Some(filename) = field.file_name() {
            // If there's a file name, it's always a file.
            let content_type = content_type.to_string();
            let name = field.name().map(|s| s.to_string()).unwrap_or_default();
            let filename = filename.to_string();
            let data = field.bytes().await?;

            files.push(FileUpload {
                name,
                filename,
                content_type,
                data: FileData(Vec::from(data)),
            });
        } else if content_type.is_form() {
            let field_name = field.name().map(|s| s.to_string());
            let data = field.bytes().await?;
            let val = value_from_urlencoded(&data);
            if let Some(name) = field_name {
                output[name] = val;
            } else {
                output = val;
            }
        } else if content_type.is_json() {
            let field_name = field.name().map(|s| s.to_string());
            let data = field.bytes().await?;
            let mut jd = serde_json::Deserializer::from_slice(&data);
            let val = serde_path_to_error::deserialize(&mut jd).map_err(Rejection::Serde)?;
            if let Some(name) = field_name {
                output[name] = val;
            } else {
                output = val;
            }
        } else if let Some(name) = field.name() {
            let name = name.to_string();
            let data = field.text().await?;
            coerce_and_push_array(&mut output, name, json!(data));
        }
    }

    Ok((output, files))
}

/// Extract a multipart form submission and perform JSON schema validation.
/// The `data` field contains all the non-file submissions, and the uploaded files
/// are placed in the `files` field.
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
        let (mut data, files) = parse_multipart(req).await?;

        super::json_schema::validate::<T>(&mut data, true).map_err(Rejection::Validation)?;

        let data = serde_path_to_error::deserialize(data).map_err(Rejection::Serde)?;

        Ok(Self { data, files })
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use super::*;

    #[tokio::test]
    async fn parse_multipart() {
        let body = indoc! {r##"
            --fieldB
            Content-Disposition: form-data; name="name"

            test
            --fieldB
            Content-Disposition: form-data; name="file1"; filename="a.txt"
            Content-Type: text/plain

            Some text
            --fieldB
            Content-Disposition: form-data; name="file2"; filename="a.html"
            Content-Type: text/html

            <b>Some html</b>
            --fieldB
            Content-Disposition: form-data; name="agreed"

            on
            --fieldB--
            "##}
        .replace("\n", "\r\n");
        println!("{}", body);

        let data = hyper::Request::builder()
            .header("content-type", "multipart/form-data; boundary=fieldB")
            .header("content-length", body.len())
            .body(axum::body::Body::from(body))
            .unwrap();

        let (value, files) = super::parse_multipart(data).await.unwrap();
        assert_eq!(
            value,
            json!({
                "name": "test",
                "agreed": "on"
            })
        );

        assert_eq!(
            files,
            vec![
                (FileUpload {
                    name: "file1".to_string(),
                    filename: "a.txt".to_string(),
                    content_type: "text/plain".to_string(),
                    data: FileData(Vec::from("Some text".as_bytes()))
                }),
                (FileUpload {
                    name: "file2".to_string(),
                    filename: "a.html".to_string(),
                    content_type: "text/html".to_string(),
                    data: FileData(Vec::from("<b>Some html</b>".as_bytes()))
                }),
            ]
        );
    }
}
