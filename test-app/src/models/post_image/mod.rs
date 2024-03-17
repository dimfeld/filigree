pub mod queries;
pub mod storage;
#[cfg(test)]
pub mod testing;
pub mod types;

pub use types::*;

pub const READ_PERMISSION: &str = "PostImage::read";
pub const WRITE_PERMISSION: &str = "PostImage::write";
pub const OWNER_PERMISSION: &str = "PostImage::owner";

pub const CREATE_PERMISSION: &str = "PostImage::owner";

filigree::make_object_id!(PostImageId, pstimg);
