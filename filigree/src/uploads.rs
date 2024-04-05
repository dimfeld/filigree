//! Helpers to manage file uploads, for use with [Storage::save_and_inspect_request_body](super::Storage::save_and_inspect_request_body).

use bytes::Bytes;
use digest::Digest;

use crate::{
    errors::{ErrorKind, HttpError},
    storage::StorageError,
};

/// An object that can inspect chunks of a stream as it is uploaded
pub trait UploadInspector<E> {
    /// Inspect a chunk of the stream
    fn inspect(&mut self, bytes: &Bytes) -> Result<(), E>;
}

/// An error that may occur while examining an upload
#[derive(Debug, thiserror::Error)]
pub enum UploadInspectorError {
    /// The file size is too large
    #[error("File size too large")]
    FileSizeTooLarge,
    /// An I/O error occurred while uploading the file
    #[error(transparent)]
    IO(#[from] StorageError),
    /// An I/O error occurred while reading the request body
    #[error(transparent)]
    Read(#[from] axum::Error),
}

impl HttpError for UploadInspectorError {
    type Detail = ();

    fn status_code(&self) -> http::StatusCode {
        match self {
            UploadInspectorError::FileSizeTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
            UploadInspectorError::IO(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            UploadInspectorError::Read(_) => http::StatusCode::BAD_REQUEST,
        }
    }

    fn error_kind(&self) -> &'static str {
        match self {
            UploadInspectorError::FileSizeTooLarge => ErrorKind::UploadTooLarge,
            UploadInspectorError::IO(_) => ErrorKind::IO,
            UploadInspectorError::Read(_) => ErrorKind::RequestRead,
        }
        .as_str()
    }

    fn error_detail(&self) -> Self::Detail {
        ()
    }
}

/// Record the size of a request body that is being uploaded, and optionally return an error
/// if the file size is too large.
pub struct UploadSize {
    size: usize,
    limit: Option<usize>,
}

impl UploadSize {
    /// Create a new UploadSize inspector
    pub fn new(limit: Option<usize>) -> Self {
        Self { size: 0, limit }
    }

    /// Return the calculated size
    pub fn finish(self) -> usize {
        self.size
    }
}

impl UploadInspector<UploadInspectorError> for UploadSize {
    fn inspect(&mut self, bytes: &Bytes) -> Result<(), UploadInspectorError> {
        self.size += bytes.len();

        let too_large = self.limit.map(|l| self.size > l).unwrap_or(false);
        if too_large {
            return Err(UploadInspectorError::FileSizeTooLarge);
        }

        Ok(())
    }
}

/// Calculate a hash of an upload
pub struct UploadHasher<D: Digest> {
    hasher: D,
}

impl<D: Digest> UploadHasher<D> {
    /// Create a new hasher of the given type.
    pub fn new() -> Self {
        Self { hasher: D::new() }
    }

    /// Return the final hash of all the chunks.
    pub fn finish(self) -> digest::Output<D> {
        self.hasher.finalize()
    }
}

impl<D: Digest> UploadInspector<UploadInspectorError> for UploadHasher<D> {
    /// Hash a chunk that is passing through
    fn inspect(&mut self, bytes: &Bytes) -> Result<(), UploadInspectorError> {
        self.hasher.update(bytes);
        Ok(())
    }
}
