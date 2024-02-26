//! Functions for parsing file uploads
use std::{fmt::Debug, ops::Deref};

use base64::Engine as _;
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize,
};

/// A wrapper for a file data which can deserialize from an array or base64-encoded string.
#[derive(Clone, PartialEq)]
pub struct FileData(pub Vec<u8>);

impl Debug for FileData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileData({} bytes)", self.0.len())
    }
}

impl FileData {
    /// Return the inner buffer from the FileData
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl Deref for FileData {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for FileData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = deserializer.deserialize_byte_buf(FileDataVisitor)?;
        Ok(FileData(v))
    }
}

/// Get file data from either a number array or base64-encoded string.
struct FileDataVisitor;

impl<'de> Visitor<'de> for FileDataVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "a byte array or base64-encoded string")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        base64::engine::general_purpose::STANDARD
            .decode(v)
            .map_err(|e| E::custom(format!("invalid base64: {}", e)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(1024));
        while let Some(el) = seq.next_element::<u8>()? {
            values.push(el);
        }

        Ok(values)
    }
}

/// A file upload from a Multipart form submission
#[derive(Deserialize, Clone, PartialEq)]
pub struct FileUpload {
    /// The name of the file control from the form
    pub name: String,
    /// The filename of the file
    pub filename: String,
    /// The content type of the file
    pub content_type: String,
    /// The file data
    pub data: FileData,
}

impl Debug for FileUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileUpload")
            .field("name", &self.name)
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field("data (bytes)", &self.data.len())
            .finish()
    }
}
