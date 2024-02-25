pub mod queries;
#[cfg(test)]
pub mod testing;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "Poll::read";
pub const WRITE_PERMISSION: &str = "Poll::write";
pub const OWNER_PERMISSION: &str = "Poll::owner";

pub const CREATE_PERMISSION: &str = "Poll::owner";

filigree::make_object_id!(PollId, pol);
