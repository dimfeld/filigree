use super::{PollCreatePayload, PollId, PollUpdatePayload};
use crate::models::post::PostId;

/// Generate a PollCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> PollCreatePayload {
    PollCreatePayload {
        id: None,
        question: format!("Test object {i}"),
        answers: serde_json::json!({ "key": i }),
        post_id: <PostId as Default>::default(),
    }
}

/// Generate a PollUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> PollUpdatePayload {
    PollUpdatePayload {
        id: None,
        question: format!("Test object {i}"),
        answers: serde_json::json!({ "key": i }),
        post_id: <PostId as Default>::default(),
    }
}
