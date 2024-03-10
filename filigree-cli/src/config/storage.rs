use filigree::storage::StorageProvider;
use serde::{Deserialize, Serialize};

/// A storage location to access.
/// Configuration for storage involves setting a provider and, for most scenarios, authentication
/// settings.
///
/// Authentication should be configured using environment variables and can be set on either
/// a storage provider level or individually for each [StorageConfig]
///
/// Storage settings are configured with this precedence:
/// - Environment variables for this particular StorageConfig, with prefix {env_prefix}STORAGE_{storage_name}_{varname}
/// - Environment variables for the storage provider, with prefix {env_prefix}STORAGE_PROVIDER_{provider_name}_{varname}
/// - The values listed in this configuration.
///
/// In this case, `env_prefix` indicates the value from the top-level configuration, if set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// The name of this storage location. It will be accessible under this name in the server
    /// state, and environment variables to configure the location will be prefixed with
    /// `STORAGE_{name}_`.
    name: String,
    /// The name of an entry in storage_providers, or one of the preconfigured providers.
    provider: String,
    /// The bucket within the storage provider to access.
    bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigStorageProvider {
    /// A known storage provider with pre-filled defaults for endpoint, virtual host style, etc.
    Preconfigured(StorageProvider),
    /// A custom storage provider, which can be set up by modifying the generated code or by
    /// setting environment variables.
    Custom(filigree::storage::StorageConfig),
}

impl ConfigStorageProvider {
    pub fn template_text(&self) -> String {
        let config = match self {
            Self::Preconfigured(provider) => {
                // recreate the provider in code
                // Might need to make more of the StorageProvider settings optional
            }
            Self::Custom(config) => {
                // recreate the config in code
            }
        };

        todo!();
    }
}
