pub mod endpoints;
pub mod queries;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "Report::read";
pub const WRITE_PERMISSION: &str = "Report::write";
pub const OWNER_PERMISSION: &str = "Report::owner";

pub const CREATE_PERMISSION: &str = "Report::owner";

filigree::make_object_id!(ReportId, rep);
