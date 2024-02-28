use super::{CommentCreatePayload, CommentId, CommentUpdatePayload};
use crate::models::post::PostId;

/// Generate a CommentCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> CommentCreatePayload {
    CommentCreatePayload {
        id: None,
        body: format!("Test object {i}"),
        post_id: <PostId as Default>::default(),
    }
}

/// Generate a CommentUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> CommentUpdatePayload {
    CommentUpdatePayload {
        id: None,
        body: format!("Test object {i}"),
        post_id: <PostId as Default>::default(),
    }
}
