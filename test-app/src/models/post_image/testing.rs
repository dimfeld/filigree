#![allow(unused_imports, unused_variables, dead_code)]
use super::{PostImageCreatePayload, PostImageId, PostImageUpdatePayload};
use crate::models::post::PostId;

/// Generate a PostImageCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> PostImageCreatePayload {
    PostImageCreatePayload {
        id: None,
        file_storage_key: format!("Test object {i}"),
        file_storage_bucket: format!("Test object {i}"),
        file_original_name: (i > 1).then(|| format!("Test object {i}")),
        file_size: (i > 1).then(|| i as i64),
        file_hash: (i > 1).then(|| <Vec<u8> as Default>::default()),
        post_id: <PostId as Default>::default(),
    }
}

/// Generate a PostImageUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> PostImageUpdatePayload {
    PostImageUpdatePayload {
        id: None,
        file_storage_key: format!("Test object {i}"),
        file_storage_bucket: format!("Test object {i}"),
        file_original_name: Some(format!("Test object {i}")),
        file_size: Some(i as i64),
        file_hash: Some(<Vec<u8> as Default>::default()),
        post_id: <PostId as Default>::default(),
    }
}
