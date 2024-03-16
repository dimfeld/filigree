//! Object storage functionality for PostImage
#![allow(unused_imports, unused_variables, dead_code)]

use bytes::Bytes;
use error_stack::ResultExt;
use filigree::{
    storage::{Storage, StorageError},
    uploads::{self, UploadInspector, UploadInspectorError},
};
use futures::stream::Stream;
use sqlx::PgConnection;

use super::{PostImage, PostImageId, PostImageUpdatePayload};
use crate::{auth::Authed, error::Error, models::post::PostId, server::ServerState};

/// Apply the storage key template
pub fn generate_object_key(auth: &Authed, id: PostImageId, filename: &str) -> String {
    format!(r##"{id}-{filename}"##, id = id, filename = filename,)
}

pub fn get_storage(state: &ServerState) -> &Storage {
    &state.storage.image_uploads
}

pub async fn upload_stream<E>(
    state: &ServerState,
    auth: &Authed,
    tx: &mut PgConnection,
    parent_id: PostId,
    id: Option<PostImageId>,
    filename: Option<String>,
    key: Option<String>,
    limit: Option<usize>,
    body: impl Stream<Item = Result<Bytes, E>> + Unpin,
) -> Result<PostImage, error_stack::Report<Error>>
where
    StorageError: From<E>,
    UploadInspectorError: From<E>,
{
    let storage = get_storage(state);

    let id = id.unwrap_or_else(PostImageId::new);

    let file_storage_key = key
        .unwrap_or_else(|| generate_object_key(auth, id, filename.as_deref().unwrap_or_default()));

    let mut file_size = uploads::UploadSize::new(limit);
    let mut hasher = uploads::UploadHasher::<blake3::Hasher>::new();

    storage
        .save_and_inspect_request_body(&file_storage_key, body, |chunk| {
            file_size.inspect(chunk)?;
            hasher.inspect(chunk)?;
            Ok::<(), UploadInspectorError>(())
        })
        .await
        .change_context(Error::Upload)?;

    let db_payload = PostImageUpdatePayload {
        id: Some(id),
        post_id: parent_id,
        file_storage_key,
        file_storage_bucket: "image_uploads".to_string(),
        file_original_name: filename,
        file_hash: Some(hasher.finish().to_vec()),
        file_size: Some(file_size.finish() as i64),
        ..Default::default()
    };

    let result =
        super::queries::upsert_with_parent(tx, auth.organization_id, true, parent_id, &db_payload)
            .await?;

    Ok(result)
}

pub async fn upload(
    state: &ServerState,
    auth: &Authed,
    tx: &mut PgConnection,
    parent_id: PostId,
    id: Option<PostImageId>,
    filename: Option<String>,
    key: Option<String>,
    limit: Option<usize>,
    body: Bytes,
) -> Result<PostImage, error_stack::Report<Error>> {
    let file_size = body.len();
    if let Some(limit) = limit {
        if file_size > limit {
            return Err(error_stack::Report::new(
                UploadInspectorError::FileSizeTooLarge,
            ))
            .change_context(Error::Upload);
        }
    }

    let b = body.clone();
    let hash = tokio::task::spawn_blocking(move || {
        let mut hasher = uploads::UploadHasher::<blake3::Hasher>::new();
        hasher.inspect(&b).ok();
        hasher.finish().to_vec()
    })
    .await
    .change_context(Error::Upload)?;

    let id = id.unwrap_or_else(PostImageId::new);

    let file_storage_key = key
        .unwrap_or_else(|| generate_object_key(auth, id, filename.as_deref().unwrap_or_default()));

    let db_payload = PostImageUpdatePayload {
        id: Some(id),
        post_id: parent_id,
        file_storage_key: file_storage_key.clone(),
        file_storage_bucket: "image_uploads".to_string(),
        file_original_name: filename,
        file_hash: Some(hash),
        file_size: Some(file_size as i64),
        ..Default::default()
    };

    let result =
        super::queries::upsert_with_parent(tx, auth.organization_id, true, parent_id, &db_payload)
            .await?;

    let storage = get_storage(state);
    storage
        .put(&file_storage_key, body)
        .await
        .change_context(Error::Upload)?;

    Ok(result)
}

/// Delete an object given the storage key
pub async fn delete_by_key(
    state: &ServerState,
    key: &str,
) -> Result<(), error_stack::Report<Error>> {
    let storage = get_storage(state);
    storage.delete(key).await.change_context(Error::Storage)?;
    Ok(())
}

/// Delete a file from the database and from object storage.
pub async fn delete_by_id(
    state: &ServerState,
    auth: &Authed,
    tx: &mut PgConnection,
    parent_id: PostId,
    id: PostImageId,
) -> Result<(), error_stack::Report<Error>> {
    let storage_key = get_storage_key_by_id(state, auth, &mut *tx, id).await?;
    let deleted = super::queries::delete_with_parent(&mut *tx, auth, parent_id, id).await?;

    if deleted {
        delete_by_key(state, &storage_key).await?;
    }

    Ok(())
}

pub async fn get_storage_key_by_id(
    state: &ServerState,
    auth: &Authed,
    tx: &mut PgConnection,
    id: PostImageId,
) -> Result<String, error_stack::Report<Error>> {
    let storage_key = super::queries::get(&mut *tx, auth, id)
        .await?
        .file_storage_key;
    Ok(storage_key)
}
