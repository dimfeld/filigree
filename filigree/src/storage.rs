use axum::body::{Body, BodyDataStream};
use bytes::Bytes;
use futures::{Future, TryFutureExt, TryStreamExt};
use object_store::{path::Path, GetResult, MultipartId, ObjectStore as _, PutResult};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::instrument;

use self::in_memory::InMemoryStore;

pub(crate) mod in_memory;
#[cfg(feature = "storage_aws")]
pub mod s3;

/// An error that may occur during a storage operation
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// I/O error while writing to storage
    #[error("I/O error: {0}")]
    StorageIo(#[from] tokio::io::Error),
    /// I/O error while reading the request body
    #[error("Request body error: {0}")]
    Body(#[from] axum::Error),
    /// Object was not found
    #[error("Object not found")]
    NotFound(#[source] object_store::Error),
    /// Storage backend error
    #[error("Storage backend error")]
    ObjectStore(#[source] object_store::Error),
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    Configuration(&'static str),
}

impl From<object_store::Error> for StorageError {
    fn from(value: object_store::Error) -> Self {
        match value {
            object_store::Error::NotFound { .. } => Self::NotFound(value),
            _ => Self::ObjectStore(value),
        }
    }
}

/// An abstraction over a storage provider for a particular bucket. This is a thin layer over
/// [object_store].
#[derive(Debug)]
pub struct Storage {
    /// The bucket managed by this Storage instance
    pub bucket: String,
    store: ObjectStore,
}

impl Storage {
    #[cfg(feature = "storage_aws")]
    /// Create a new Storage for an S3 bucket
    pub fn new_s3(options: &s3::S3StoreConfig, bucket: String) -> Result<Self, StorageError> {
        let store = s3::create_store(options, &bucket)?;
        Ok(Self {
            bucket,
            store: ObjectStore::S3(store),
        })
    }

    /// Create a new Storage for a local filesystem
    pub fn new_local(base_path: String) -> Result<Self, object_store::Error> {
        let store = object_store::local::LocalFileSystem::new_with_prefix(&base_path)?;
        Ok(Self {
            bucket: base_path,
            store: ObjectStore::Local(store),
        })
    }

    /// Create a new in-memory store for testing
    pub fn new_memory() -> Self {
        Self {
            bucket: "memory".to_string(),
            store: ObjectStore::Memory(InMemoryStore::new()),
        }
    }

    /// Get an object from the store
    #[instrument]
    pub async fn get(&self, location: &str) -> Result<GetResult, object_store::Error> {
        let location = Path::from(location);
        self.store.get(&location).await
    }

    /// Write an object to the store
    #[instrument(skip(bytes))]
    pub async fn put(
        &self,
        location: &str,
        bytes: Bytes,
    ) -> Result<PutResult, object_store::Error> {
        let location = Path::from(location);
        self.store.put(&location, bytes).await
    }

    /// Write an object to the store as a stream
    #[instrument]
    pub async fn put_multipart(
        &self,
        location: &str,
    ) -> Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>), object_store::Error> {
        let location = Path::from(location);
        self.store.put_multipart(&location).await
    }

    /// Delete an object from the store
    #[instrument]
    pub async fn delete(&self, location: &str) -> Result<(), object_store::Error> {
        let location = Path::from(location);
        self.store.delete(&location).await
    }

    /// Abort a streaming upload. This tells the storage provider to drop any chunks that have been
    /// uploaded so far.
    #[instrument]
    pub async fn abort_multipart(
        &self,
        location: &str,
        id: &MultipartId,
    ) -> Result<(), object_store::Error> {
        let location = Path::from(location);
        self.store.abort_multipart(&location, id).await
    }

    /// Return true if the storage provider supports presigned upload URLs.
    pub fn supports_presigned_urls(&self) -> bool {
        self.store.supports_presigned_urls()
    }

    /// Stream an object from the store to a [Response]
    pub async fn stream_to_client(&self, location: &str) -> Result<Body, object_store::Error> {
        let stream = self.get(location).await?.into_stream();
        Ok(Body::from_stream(stream))
    }

    /// Stream a request body into object storage
    pub async fn save_request_body<E, F, Fut>(
        &self,
        location: &str,
        body: BodyDataStream,
    ) -> Result<usize, StorageError> {
        self.save_and_inspect_request_body(
            location,
            body,
            |_| async move { Ok::<_, StorageError>(()) },
        )
        .await
    }

    /// Stream a request body into object storage
    pub async fn save_and_inspect_request_body<E, F, Fut>(
        &self,
        location: &str,
        body: BodyDataStream,
        inspect: F,
    ) -> Result<usize, E>
    where
        E: From<StorageError>,
        F: FnMut(&Bytes) -> Fut,
        Fut: Future<Output = Result<(), E>>,
    {
        let (upload_id, mut writer) = self
            .put_multipart(location)
            .await
            .map_err(StorageError::from)?;
        let result = self.upload_body(&mut writer, body, inspect).await;
        if let Err(_) = &result {
            self.abort_multipart(location, &upload_id).await.ok();
        }

        result
    }

    async fn upload_body<E, F, Fut>(
        &self,
        upload: &mut Box<dyn AsyncWrite + Unpin + Send>,
        mut stream: BodyDataStream,
        mut inspect: F,
    ) -> Result<usize, E>
    where
        E: From<StorageError>,
        F: FnMut(&Bytes) -> Fut,
        Fut: Future<Output = Result<(), E>>,
    {
        let mut total_size = 0;
        while let Some(chunk) = stream.try_next().await.map_err(StorageError::from)? {
            total_size += chunk.len();
            tokio::try_join!(
                inspect(&chunk),
                upload
                    .write_all(&chunk)
                    .map_err(|e| E::from(StorageError::from(e)))
            )?;
        }

        upload.shutdown().await.map_err(StorageError::from)?;

        Ok(total_size)
    }
}

/// Dispatch to different stores. We use this instead of a dyn trait so that we can get both
/// [ObjectStore] and [Signer] methods, for providers that support them.
enum ObjectStore {
    Local(object_store::local::LocalFileSystem),
    S3(object_store::aws::AmazonS3),
    Memory(InMemoryStore),
}

impl std::fmt::Debug for ObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(_) => f.debug_tuple("Local").finish(),
            Self::S3(_) => f.debug_tuple("S3").finish(),
            Self::Memory(_) => f.debug_tuple("Memory").finish(),
        }
    }
}

impl ObjectStore {
    pub async fn get(&self, location: &Path) -> object_store::Result<GetResult> {
        match self {
            ObjectStore::Local(local) => local.get(location).await,
            ObjectStore::S3(s3) => s3.get(location).await,
            ObjectStore::Memory(mem) => mem.get(location).await,
        }
    }

    pub async fn put(&self, location: &Path, data: Bytes) -> object_store::Result<PutResult> {
        match self {
            ObjectStore::Local(local) => local.put(location, data).await,
            ObjectStore::S3(s3) => s3.put(location, data).await,
            ObjectStore::Memory(mem) => mem.put(location, data).await,
        }
    }

    pub async fn put_multipart(
        &self,
        location: &Path,
    ) -> object_store::Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>)> {
        match self {
            ObjectStore::Local(local) => local.put_multipart(location).await,
            ObjectStore::S3(s3) => s3.put_multipart(location).await,
            ObjectStore::Memory(mem) => mem.put_multipart(location).await,
        }
    }

    pub async fn abort_multipart(
        &self,
        location: &Path,
        id: &MultipartId,
    ) -> object_store::Result<()> {
        match self {
            ObjectStore::Local(local) => local.abort_multipart(location, id).await,
            ObjectStore::S3(s3) => s3.abort_multipart(location, id).await,
            ObjectStore::Memory(_) => Ok(()),
        }
    }

    pub async fn delete(&self, location: &Path) -> object_store::Result<()> {
        match self {
            ObjectStore::Local(local) => local.delete(location).await,
            ObjectStore::S3(s3) => s3.delete(location).await,
            ObjectStore::Memory(mem) => mem.delete(location).await,
        }
    }

    pub fn supports_presigned_urls(&self) -> bool {
        matches!(&self, ObjectStore::S3(_))
    }
}
