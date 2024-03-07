use bytes::Bytes;
use error_stack::{Report, ResultExt};
use object_store::{path::Path, GetResult, MultipartId, ObjectStore as _, PutResult};
use thiserror::Error;
use tokio::io::AsyncWrite;
use tracing::instrument;

#[cfg(feature = "storage_aws")]
pub mod s3;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Failed to build client")]
    BuildingClient,
    #[error("access_key_id and secret_key must be both set or both unset")]
    AccessAndSecretKey,
    #[error("Failed to get {0}")]
    Get(Path),
    #[error("Failed to put {0}")]
    Put(Path),
    #[error("Failed to start multipart put to {0}")]
    PutMultipart(Path),
    #[error("Failed to abort multipart put to {0}")]
    AbortMultipart(Path),
}

pub struct Storage {
    /// The bucket managed by this Storage instance
    pub bucket: String,
    store: ObjectStore,
}

impl Storage {
    #[cfg(feature = "storage_aws")]
    /// Create a new Storage for an S3 bucket
    pub fn new_s3(
        options: &s3::S3StoreConfig,
        bucket: String,
    ) -> Result<Self, Report<StorageError>> {
        let store = s3::create_store(options, &bucket)?;
        Ok(Self {
            bucket,
            store: ObjectStore::S3(store),
        })
    }

    /// Create a new Storage for a local filesystem
    pub fn new_local(base_path: String) -> Result<Self, Report<StorageError>> {
        let store = object_store::local::LocalFileSystem::new_with_prefix(&base_path)
            .change_context(StorageError::BuildingClient)?;
        Ok(Self {
            bucket: base_path,
            store: ObjectStore::Local(store),
        })
    }

    #[instrument(skip(self), fields(bucket=%self.bucket))]
    pub async fn get(&self, location: &str) -> Result<GetResult, Report<StorageError>> {
        let location = Path::from(location);
        self.store
            .get(&location)
            .await
            .change_context(StorageError::Get(location))
    }

    #[instrument(skip(self, bytes), fields(bucket=%self.bucket))]
    pub async fn put(&self, location: &str, bytes: Bytes) -> Result<(), Report<StorageError>> {
        let location = Path::from(location);
        self.store
            .put(&location, bytes)
            .await
            .change_context(StorageError::Put(location))?;
        Ok(())
    }

    #[instrument(skip(self), fields(bucket=%self.bucket))]
    pub async fn put_multipart(
        &self,
        location: &str,
    ) -> Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>), Report<StorageError>> {
        let location = Path::from(location);
        self.store
            .put_multipart(&location)
            .await
            .change_context(StorageError::PutMultipart(location))
    }

    #[instrument(skip(self))]
    pub async fn abort_multipart(
        &self,
        location: &str,
        id: &MultipartId,
    ) -> Result<(), Report<StorageError>> {
        let location = Path::from(location);
        self.store
            .abort_multipart(&location, id)
            .await
            .change_context(StorageError::AbortMultipart(location))
    }

    pub fn supports_signed_urls(&self) -> bool {
        self.store.supports_signed_urls()
    }
}

/// Dispatch to different stores. We use this instead of a dyn trait so that we can get both
/// [ObjectStore] and [Signer] methods, for providers that support them.
enum ObjectStore {
    Local(object_store::local::LocalFileSystem),
    S3(object_store::aws::AmazonS3),
}

impl ObjectStore {
    pub async fn get(&self, location: &Path) -> object_store::Result<GetResult> {
        match self {
            ObjectStore::Local(local) => local.get(location).await,
            ObjectStore::S3(s3) => s3.get(location).await,
        }
    }

    pub async fn put(&self, location: &Path, data: Bytes) -> object_store::Result<PutResult> {
        match self {
            ObjectStore::Local(local) => local.put(location, data).await,
            ObjectStore::S3(s3) => s3.put(location, data).await,
        }
    }

    pub async fn put_multipart(
        &self,
        location: &Path,
    ) -> object_store::Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>)> {
        match self {
            ObjectStore::Local(local) => local.put_multipart(location).await,
            ObjectStore::S3(s3) => s3.put_multipart(location).await,
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
        }
    }

    pub fn supports_signed_urls(&self) -> bool {
        matches!(&self, ObjectStore::S3(_))
    }
}
