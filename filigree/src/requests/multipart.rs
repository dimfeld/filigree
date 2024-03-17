//! Parse multipart form requests
use std::marker::PhantomData;

use axum::{
    body::Body,
    extract::{FromRequest, Request},
};
use axum_extra::extract::multipart::Field;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::file::{FileData, FileUpload};
use crate::extract::Rejection;

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

enum MultipartField {
    File(Field),
    Data(String, String),
}

async fn handle_multipart_field(field: Field) -> Result<MultipartField, Rejection> {
    let value = if field.file_name().is_some() {
        // If there's a file name, it's always a file.
        MultipartField::File(field)
    } else if let Some(name) = field.name() {
        let name = name.to_string();
        let data = field.text().await?;
        MultipartField::Data(name, data)
    } else {
        // If it doesn't have a filename or a name, it's probably a weird submission of a file.
        // Let the caller figure out if that's valid or not in this case.
        MultipartField::File(field)
    };

    Ok(value)
}

/// Parse a multipart form submission into the specified type and a list of files uploaded with it.
pub async fn parse_multipart<T>(req: Request<Body>) -> Result<(T, Vec<FileUpload>), Rejection>
where
    T: DeserializeOwned + JsonSchema + Send + Sync + 'static,
{
    let mut files = Vec::new();

    let multipart = axum_extra::extract::Multipart::from_request(req, &())
        .await
        .map_err(Rejection::Multipart)?;

    let mut processor = MultipartProcessor::<T>::from(multipart);

    while let Some(field) = processor.next_file().await? {
        let content_type = field.content_type().unwrap_or_default().to_string();
        let name = field.name().map(|s| s.to_string()).unwrap_or_default();
        let filename = field.file_name().unwrap_or_default().to_string();
        let data = field.bytes().await?;

        files.push(FileUpload {
            name,
            filename,
            content_type,
            data: FileData(Vec::from(data)),
        });
    }

    let output = processor.finish().await?;

    Ok((output, files))
}

/// Iterate over a multipart submission, returning the files for processing
/// and accumulating the other fields encountered on the way. When done, call
/// [finish] to validate and deserialize the accumulated non-file fields.
///
/// ```ignore
/// # use axum::{extract::{ FromRequest, Request}, RequestExt};
/// # use filigree::{requests::{multipart::MultipartProcessor}, extract::Rejection};
/// # #[derive(serde::Deserialize, schemars::JsonSchema)]
/// # struct T {}
/// # async fn example(req: Request) -> Result<(), Rejection> {
/// let multipart = axum_extra::extract::Multipart::from_request(req, &())
///     .await?;
///
/// let mut processor = MultipartProcessor::from(multipart);
///
/// while let Some(field) = processor.next_file().await? {
///     let content_type = field.content_type().unwrap_or_default().to_string();
///     let name = field.name().map(|s| s.to_string()).unwrap_or_default();
///     let filename = field.file_name().unwrap_or_default().to_string();
///
///     // Upload the file or do something with it here.
/// }
///
/// // And get the rest of the form info.
/// let output: T = processor.finish().await?;
/// # Ok::<_, Rejection>(())
/// # }
/// ```
pub struct MultipartProcessor<T>
where
    T: DeserializeOwned + JsonSchema + Send + Sync + 'static,
{
    multipart: axum_extra::extract::Multipart,
    data: serde_json::Value,
    _marker: PhantomData<T>,
    /// If false (the default), then the processor will return an error if it encounters
    /// a file upload in a call to `finish`. If true, the file will be silently skipped.
    pub may_skip_files: bool,
}

impl<T> MultipartProcessor<T>
where
    T: DeserializeOwned + JsonSchema + Send + Sync + 'static,
{
    /// Iterate over the fields in the multipart form, returning the next file field
    /// and internally accumulating the other fields encountered on the way.
    pub async fn next_file(&mut self) -> Result<Option<Field>, Rejection> {
        while let Some(field) = self.multipart.next_field().await? {
            match handle_multipart_field(field).await? {
                MultipartField::File(field) => {
                    return Ok(Some(field));
                }
                MultipartField::Data(key, value) => {
                    coerce_and_push_array(&mut self.data, key, json!(value));
                }
            }
        }

        Ok(None)
    }

    /// Finish parsing the multipart form into the specified type, and return
    /// the deserialized, non-file fields.
    pub async fn finish(mut self) -> Result<T, Rejection> {
        while let Some(field) = self.multipart.next_field().await? {
            match handle_multipart_field(field).await? {
                MultipartField::File(_) => {
                    if !self.may_skip_files {
                        return Err(Rejection::TooManyFiles);
                    }
                }
                MultipartField::Data(key, value) => {
                    coerce_and_push_array(&mut self.data, key, json!(value));
                }
            }
        }

        let data = crate::requests::json_schema::validate::<T>(self.data, true)
            .map_err(Rejection::Validation)?;

        serde_path_to_error::deserialize(data).map_err(Rejection::Serde)
    }
}

impl<T> From<axum_extra::extract::Multipart> for MultipartProcessor<T>
where
    T: DeserializeOwned + JsonSchema + Send + Sync + 'static,
{
    fn from(multipart: axum_extra::extract::Multipart) -> Self {
        Self {
            multipart,
            data: json!({}),
            _marker: PhantomData,
            may_skip_files: false,
        }
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;
    use serde::Deserialize;

    use super::*;

    fn get_req() -> hyper::Request<axum::body::Body> {
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

        hyper::Request::builder()
            .header("content-type", "multipart/form-data; boundary=fieldB")
            .header("content-length", body.len())
            .body(axum::body::Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn parse_multipart_jsonvalue() {
        let data = get_req();
        let (value, files) = super::parse_multipart::<serde_json::Value>(data)
            .await
            .unwrap();
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

    #[tokio::test]
    async fn parse_multipart_serde() {
        #[derive(Deserialize, JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            name: String,
            agreed: bool,
        }

        let data = get_req();
        let (value, files) = super::parse_multipart::<Data>(data).await.unwrap();

        assert_eq!(
            value,
            Data {
                name: "test".to_string(),
                agreed: true
            }
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

    #[tokio::test]
    async fn multipart_processor() {
        let req = get_req();

        #[derive(Deserialize, JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            name: String,
            agreed: bool,
        }

        let multipart = axum_extra::extract::Multipart::from_request(req, &())
            .await
            .unwrap();
        let mut processor = super::MultipartProcessor::<Data>::from(multipart);

        let mut index = 0;
        while let Some(file) = processor.next_file().await.unwrap() {
            match index {
                0 => {
                    assert_eq!(file.file_name(), Some("a.txt"));
                    assert_eq!(file.content_type(), Some("text/plain"));
                    assert_eq!(file.name(), Some("file1"));
                    assert_eq!(file.text().await.unwrap(), "Some text");
                }
                1 => {
                    assert_eq!(file.file_name(), Some("a.html"));
                    assert_eq!(file.content_type(), Some("text/html"));
                    assert_eq!(file.name(), Some("file2"));
                    assert_eq!(file.text().await.unwrap(), "<b>Some html</b>");
                }
                _ => panic!("Saw too many files"),
            };

            index += 1;
        }

        let output = processor.finish().await.unwrap();
        assert_eq!(
            output,
            Data {
                name: "test".to_string(),
                agreed: true
            }
        );
    }

    #[tokio::test]
    async fn multipart_processor_too_many_uploads() {
        let req = get_req();

        #[derive(Deserialize, JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            name: String,
            agreed: bool,
        }

        let multipart = axum_extra::extract::Multipart::from_request(req, &())
            .await
            .unwrap();
        let mut processor = super::MultipartProcessor::<Data>::from(multipart);

        let file = processor.next_file().await.unwrap().unwrap();
        assert_eq!(file.file_name(), Some("a.txt"));
        assert_eq!(file.content_type(), Some("text/plain"));
        assert_eq!(file.name(), Some("file1"));
        assert_eq!(file.text().await.unwrap(), "Some text");

        let err = processor.finish().await.expect_err("Finishing");
        assert!(matches!(err, Rejection::TooManyFiles));
    }

    /// Make sure it works to skip a file
    #[tokio::test]
    async fn multipart_processor_may_skip_files() {
        let req = get_req();

        #[derive(Deserialize, JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            name: String,
            agreed: bool,
        }

        let multipart = axum_extra::extract::Multipart::from_request(req, &())
            .await
            .unwrap();
        let mut processor = super::MultipartProcessor::<Data>::from(multipart);
        processor.may_skip_files = true;

        let file = processor.next_file().await.unwrap().unwrap();
        assert_eq!(file.file_name(), Some("a.txt"));
        assert_eq!(file.content_type(), Some("text/plain"));
        assert_eq!(file.name(), Some("file1"));
        assert_eq!(file.text().await.unwrap(), "Some text");

        let output = processor.finish().await.unwrap();
        assert_eq!(
            output,
            Data {
                name: "test".to_string(),
                agreed: true
            }
        );
    }
}
