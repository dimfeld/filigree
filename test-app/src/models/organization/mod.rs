pub mod endpoints;
pub mod queries;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "Organization::read";
pub const WRITE_PERMISSION: &str = "Organization::write";
pub const OWNER_PERMISSION: &str = "Organization::owner";

pub type OrganizationId = filigree::auth::OrganizationId;
