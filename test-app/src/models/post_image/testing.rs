use super::{PostImageCreatePayload, PostImageId, PostImageUpdatePayload};
use crate::models::post::PostId;

/// Generate a PostImageCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> PostImageCreatePayload {
    PostImageCreatePayload {
        id: None,
        post_id: <PostId as Default>::default(),
    }
}

/// Generate a PostImageUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> PostImageUpdatePayload {
    PostImageUpdatePayload {
        id: None,
        post_id: <PostId as Default>::default(),
    }
}
