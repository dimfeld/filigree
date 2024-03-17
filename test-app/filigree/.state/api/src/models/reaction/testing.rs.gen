#![allow(unused_imports, unused_variables, dead_code)]
use super::{ReactionCreatePayload, ReactionId, ReactionUpdatePayload};
use crate::models::post::PostId;

/// Generate a ReactionCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> ReactionCreatePayload {
    ReactionCreatePayload {
        id: None,
        typ: format!("Test object {i}"),
        post_id: <PostId as Default>::default(),
    }
}

/// Generate a ReactionUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> ReactionUpdatePayload {
    ReactionUpdatePayload {
        id: None,
        typ: format!("Test object {i}"),
        post_id: <PostId as Default>::default(),
    }
}
