pub mod endpoints;
pub mod queries;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "User::read";
pub const WRITE_PERMISSION: &str = "User::write";
pub const OWNER_PERMISSION: &str = "User::owner";

pub type UserId = filigree::auth::UserId;
