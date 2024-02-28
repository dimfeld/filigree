pub mod queries;
#[cfg(test)]
pub mod testing;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "Comment::read";
pub const WRITE_PERMISSION: &str = "Comment::write";
pub const OWNER_PERMISSION: &str = "Comment::owner";

pub const CREATE_PERMISSION: &str = "Comment::owner";

filigree::make_object_id!(CommentId, cmt);
