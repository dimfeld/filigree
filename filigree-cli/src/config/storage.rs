use filigree::storage::StorageProvider;
use serde::{Deserialize, Serialize};

/// A storage location to access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    /// The name of this storage location. It will be accessible under this name in the server
    /// state, and environment variables to configure the location will be prefixed with
    /// `STORAGE_{name}_`.
    name: String,
    /// The name of an entry in storage_providers, or one of the preconfigured providers.
    provider: StorageProvider,
    /// The bucket within the storage provider to access.
    bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigStorageProvider {
    Preconfigured(StorageProvider),
    Custom { name: String },
}
