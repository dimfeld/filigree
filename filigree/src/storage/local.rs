//! Configuration for file-system based storage
use serde::{Deserialize, Serialize};

use super::StorageError;
use crate::prefixed_env_var;

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

    pub fn from_env(prefix: &str) -> Result<Self, StorageError> {
        Ok(Self {
            base_path: prefixed_env_var(prefix, "LOCAL_BASE_PATH").ok(),
        })
    }

    pub fn merge_env(&mut self, prefix: &str) -> Result<(), StorageError> {
        let from_env = Self::from_env(prefix)?;

        if from_env.base_path.is_some() {
            self.base_path = from_env.base_path;
        }

        Ok(())
    }
}
