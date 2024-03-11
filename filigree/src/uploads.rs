//! Helpers to manage file uploads, for use with [Storage::save_and_inspect_request_body].
//! A lot of this feels silly, such as [UploadSize] which just sums the size of the stream,
//! but it simplifies generation through the template system.

use bytes::Bytes;
use digest::Digest;

use crate::{
    errors::{ErrorKind, HttpError},
    storage::StorageError,
};

/// An error that may occur while examining an upload
#[derive(Debug, thiserror::Error)]
pub enum UploadInspectorError {
    /// The file size is too large
    #[error("File size too large")]
    FileSizeTooLarge,
    /// An I/O error occurred while uploading the file
    #[error(transparent)]
    IO(#[from] StorageError),
}

impl HttpError for UploadInspectorError {
    type Detail = ();

    fn status_code(&self) -> http::StatusCode {
        match self {
            UploadInspectorError::FileSizeTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
            UploadInspectorError::IO(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_kind(&self) -> &'static str {
        match self {
            UploadInspectorError::FileSizeTooLarge => ErrorKind::UploadTooLarge,
            UploadInspectorError::IO(_) => ErrorKind::IO,
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

    /// Add the size of a chunk.
    pub async fn inspect(&mut self, bytes: &Bytes) -> Result<(), UploadInspectorError> {
        self.size += bytes.len();

        let too_large = self.limit.map(|l| self.size > l).unwrap_or(false);
        if too_large {
            return Err(UploadInspectorError::FileSizeTooLarge);
        }

        Ok(())
    }

    /// Return the calculated size
    pub fn finish(self) -> usize {
        self.size
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

    /// Hash a chunk that is passing through
    pub async fn inspect(&mut self, bytes: &Bytes) -> Result<(), UploadInspectorError> {
        self.hasher.update(bytes);
        Ok(())
    }

    /// Return the final hash of all the chunks.
    pub fn finish(self) -> digest::Output<D> {
        self.hasher.finalize()
    }
}
