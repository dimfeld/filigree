pub mod endpoints;
pub mod queries;
#[cfg(test)]
pub mod testing;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "Post::read";
pub const WRITE_PERMISSION: &str = "Post::write";
pub const OWNER_PERMISSION: &str = "Post::owner";

pub const CREATE_PERMISSION: &str = "Post::owner";

filigree::make_object_id!(PostId, pst);
