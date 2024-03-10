//! Configuration for file-system based storage
use serde::{Deserialize, Serialize};

use super::StorageError;
use crate::config::{merge_option_if_set, prefixed_env_var};

/// Configuration for file-system based storage
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LocalStoreConfig {
    /// Where the files should be stored
    pub base_path: Option<String>,
}

impl LocalStoreConfig {
    /// Create a new LocalStoreConfig
    pub fn new(base_path: Option<String>) -> Self {
        Self { base_path }
    }

    /// Initialize a [LocalStoreConfig] from environment variables
    pub fn from_env(prefix: &str) -> Result<Self, StorageError> {
        Ok(Self {
            base_path: prefixed_env_var(prefix, "LOCAL_BASE_PATH").ok(),
        })
    }

    /// Overwrite this configuration's values with environment values, if set.
    pub fn merge_env(&mut self, prefix: &str) -> Result<(), StorageError> {
        let from_env = Self::from_env(prefix)?;
        merge_option_if_set(&mut self.base_path, from_env.base_path);

        Ok(())
    }

    #[cfg(feature = "filigree-cli")]
    /// Recreate the structure in Rust code.
    pub fn template_text(&self) -> String {
        format!("LocalStoreConfig {{ base_path: {:?} }}", self.base_path)
    }
}
