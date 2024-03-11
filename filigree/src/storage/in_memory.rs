//! In-memory object store for testing

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use bytes::{Bytes, BytesMut};
use object_store::{path::Path, GetResult, GetResultPayload, MultipartId, ObjectMeta, PutResult};
use tokio::io::AsyncWrite;

#[derive(Debug)]
struct InMemoryError;

impl std::error::Error for InMemoryError {}

impl std::fmt::Display for InMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InMemoryError")
    }
}

/// In-memory object store for tests. This is intended for simplicity and is not
/// optimized for production use.
pub struct InMemoryStore {
    store: Arc<Mutex<BTreeMap<Path, (chrono::DateTime<chrono::Utc>, Bytes)>>>,
}

impl InMemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Simulate a new object being written
    pub async fn put(&self, location: &Path, bytes: Bytes) -> object_store::Result<PutResult> {
        let mut store = self.store.lock().unwrap();
        store.insert(location.clone(), (chrono::Utc::now(), bytes));
        Ok(PutResult {
            e_tag: None,
            version: None,
        })
    }

    pub async fn get(&self, location: &Path) -> object_store::Result<GetResult> {
        let store = self.store.lock().unwrap();
        let (last_modified, data) = store
            .get(location)
            .ok_or_else(|| object_store::Error::NotFound {
                path: location.to_string(),
                source: Box::new(InMemoryError),
            })?
            .clone();

        let chunk1_end = data.len() / 3;
        let chunk2_end = data.len() * 2 / 3;
        // Simulate the data coming back in a stream.
        let data_chunks = vec![
            Ok(data.slice(0..chunk1_end)),
            Ok(data.slice(chunk1_end..chunk2_end)),
            Ok(data.slice(chunk2_end..)),
        ];

        Ok(GetResult {
            range: 0..data.len(),
            meta: ObjectMeta {
                size: data.len(),
                e_tag: None,
                version: None,
                last_modified,
                location: location.clone(),
            },
            payload: GetResultPayload::Stream(Box::pin(futures::stream::iter(data_chunks))),
        })
    }

    pub async fn delete(&self, location: &Path) -> object_store::Result<()> {
        let mut store = self.store.lock().unwrap();
        store.remove(location);
        Ok(())
    }

    pub async fn put_multipart(
        &self,
        location: &Path,
    ) -> object_store::Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>)> {
        let writer = MultipartWriter {
            location: location.clone(),
            acc: BytesMut::new(),
            store: self.store.clone(),
        };

        Ok((location.to_string(), Box::new(writer)))
    }
}

/// An in-memory implementation of a multipart writer
struct MultipartWriter {
    location: Path,
    acc: BytesMut,
    store: Arc<Mutex<BTreeMap<Path, (chrono::DateTime<chrono::Utc>, Bytes)>>>,
}

impl AsyncWrite for MultipartWriter {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        self.acc.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let data = std::mem::take(&mut self.acc);
        let mut store = self.store.lock().unwrap();
        store.insert(self.location.clone(), (chrono::Utc::now(), data.freeze()));
        std::task::Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn basic_ops() {
        let store = super::InMemoryStore::new();
        store
            .put(&object_store::path::Path::from("foo"), Bytes::from("bar"))
            .await
            .unwrap();

        let get_result = store
            .get(&object_store::path::Path::from("foo"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        assert_eq!(get_result, Bytes::from("bar"));
        store
            .delete(&object_store::path::Path::from("foo"))
            .await
            .unwrap();

        let get_result = store
            .get(&object_store::path::Path::from("foo"))
            .await
            .expect_err("Get after delete should fail");
        assert!(matches!(get_result, object_store::Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn multipart_upload() {
        let store = super::InMemoryStore::new();
        let (id, mut writer) = store
            .put_multipart(&object_store::path::Path::from("foo"))
            .await
            .unwrap();
        assert_eq!(id, "foo");
        writer.write_all(&[1, 2, 3]).await.unwrap();
        writer.write_all(&[4, 5, 6]).await.unwrap();
        writer.shutdown().await.unwrap();

        let get_result = store
            .get(&object_store::path::Path::from("foo"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(get_result, Bytes::from(vec![1, 2, 3, 4, 5, 6]));
    }

    #[tokio::test]
    async fn multipart_upload_abort() {
        let store = super::InMemoryStore::new();
        let (id, mut writer) = store
            .put_multipart(&object_store::path::Path::from("foo"))
            .await
            .unwrap();
        assert_eq!(id, "foo");
        writer.write_all(&[1, 2, 3]).await.unwrap();
        // Abort for the in-memory store just means that we dropped the writer.
        drop(writer);

        let get_result = store
            .get(&object_store::path::Path::from("foo"))
            .await
            .expect_err("Get without finishing should fail");
        assert!(matches!(get_result, object_store::Error::NotFound { .. }));
    }
}
