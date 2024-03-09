//! Configuration for file-system based storage
use serde::{Deserialize, Serialize};

/// Configuration for file-system based storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStoreConfig {
    /// Where the files should be stored
    pub base_path: String,
}
