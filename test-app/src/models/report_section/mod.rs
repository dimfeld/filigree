pub mod queries;
#[cfg(test)]
pub mod testing;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "ReportSection::read";
pub const WRITE_PERMISSION: &str = "ReportSection::write";
pub const OWNER_PERMISSION: &str = "ReportSection::owner";

pub const CREATE_PERMISSION: &str = "ReportSection::owner";

filigree::make_object_id!(ReportSectionId, repsec);
