use axum::{
    body::{Body, BodyDataStream},
    extract::Request,
    response::Response,
};
use bytes::Bytes;
use futures::TryStreamExt;
use object_store::{path::Path, GetResult, MultipartId, ObjectStore as _, PutResult};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::instrument;

#[cfg(feature = "storage_aws")]
pub mod s3;

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
    pub fn new_s3(
        options: &s3::S3StoreConfig,
        bucket: String,
    ) -> Result<Self, object_store::Error> {
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

    /// Get an object from the store
    #[instrument(skip(self), fields(bucket=%self.bucket))]
    pub async fn get(&self, location: &str) -> Result<GetResult, object_store::Error> {
        let location = Path::from(location);
        self.store.get(&location).await
    }

    /// Write an object to the store
    #[instrument(skip(self, bytes), fields(bucket=%self.bucket))]
    pub async fn put(
        &self,
        location: &str,
        bytes: Bytes,
    ) -> Result<PutResult, object_store::Error> {
        let location = Path::from(location);
        self.store.put(&location, bytes).await
    }

    /// Write an object to the store as a stream
    #[instrument(skip(self), fields(bucket=%self.bucket))]
    pub async fn put_multipart(
        &self,
        location: &str,
    ) -> Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>), object_store::Error> {
        let location = Path::from(location);
        self.store.put_multipart(&location).await
    }

    /// Abort a streaming upload. This tells the storage provider to drop any chunks that have been
    /// uploaded so far.
    #[instrument(skip(self))]
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
    pub async fn save_request_body(
        &self,
        location: &str,
        body: BodyDataStream,
    ) -> Result<usize, object_store::Error> {
        let (upload_id, mut writer) = self.put_multipart(location).await?;
        let result = self.handle_upload(&mut writer, body).await;
        if let Err(_) = &result {
            self.abort_multipart(location, &upload_id).await.ok();
        }

        result
    }

    async fn handle_upload(
        &self,
        upload: &mut Box<dyn AsyncWrite + Unpin + Send>,
        mut stream: BodyDataStream,
    ) -> Result<usize, object_store::Error> {
        let mut total_size = 0;
        while let Some(chunk) = stream.try_next().await? {
            total_size += chunk.len();
            upload.write_all(&chunk).await?;
        }

        upload.shutdown().await?;

        Ok(total_size)
    }
}

/// Dispatch to different stores. We use this instead of a dyn trait so that we can get both
/// [ObjectStore] and [Signer] methods, for providers that support them.
enum ObjectStore {
    Local(object_store::local::LocalFileSystem),
    S3(object_store::aws::AmazonS3),
}

impl std::fmt::Debug for ObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(_) => f.debug_tuple("Local").finish(),
            Self::S3(_) => f.debug_tuple("S3").finish(),
        }
    }
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

    pub fn supports_presigned_urls(&self) -> bool {
        matches!(&self, ObjectStore::S3(_))
    }
}
