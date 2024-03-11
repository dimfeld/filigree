use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use filigree::storage::StoragePreset;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageConfigError {
    #[error("Bucket {bucket} references unknown storage provider {provider}")]
    UnknownProvider { bucket: String, provider: String },
    #[error("Bucket {bucket} does not reference a storage provider")]
    ProviderRequired { bucket: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage buckets
    /// The key is the name inside the application code for this storage location.
    /// It will be accessible under this name in the server state, and environment
    /// variables to configure the location will be prefixed with
    /// `{env_prefix}STORAGE_{name}_`.
    #[serde(default)]
    pub bucket: BTreeMap<String, StorageBucketConfig>,

    /// Storage providers, if not using the preconfigured options.
    #[serde(default)]
    pub provider: BTreeMap<String, StorageProviderConfig>,
}

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
pub struct StorageBucketConfig {
    /// The name of an entry in storage_providers, or one of the preconfigured providers.
    /// This can be omitted if there is only a single provider.
    provider: Option<String>,
    /// The name of the bucket within the storage provider
    bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StorageProviderConfig {
    /// A known storage provider with pre-filled defaults for endpoint, virtual host style, etc.
    Preconfigured(StoragePreset),
    /// A custom storage provider, which can be set up by modifying the generated code or by
    /// setting environment variables.
    Custom(filigree::storage::StorageConfig),
}

impl StorageProviderConfig {
    /// Regenerate this structure as Rust code
    pub fn template_text(&self) -> String {
        match self {
            Self::Preconfigured(provider) => provider.template_text(),
            Self::Custom(config) => config.template_text(),
        }
    }
}

impl StorageConfig {
    pub fn template_context(&self) -> Result<serde_json::Value, StorageConfigError> {
        if self.bucket.is_empty() && self.provider.is_empty() {
            return Ok(json!(null));
        }

        let configs = self
            .provider
            .iter()
            .map(|(name, provider)| {
                serde_json::json!({
                    "name": name.to_case(Case::Snake),
                    "name_upper": name.to_case(Case::ScreamingSnake),
                    "config_struct": provider.template_text(),
                    "is_preset": matches!(provider, StorageProviderConfig::Preconfigured(_))
                })
            })
            .collect::<Vec<_>>();

        let can_omit_provider = self.provider.len() == 1;
        let buckets = self
            .bucket
            .iter()
            .map(|(name, bucket)| {
                let provider_name = match (bucket.provider.as_deref(), can_omit_provider) {
                    // If there's only one provider, then just use that one.
                    (None, true) => self
                        .provider
                        .iter()
                        .map(|(k, _)| k.as_str())
                        .next()
                        .unwrap(),
                    (None, false) => Err(StorageConfigError::ProviderRequired {
                        bucket: bucket.bucket.clone(),
                    })?,
                    (Some(provider_name), _) => {
                        if !self.provider.contains_key(provider_name) {
                            return Err(StorageConfigError::UnknownProvider {
                                bucket: name.to_string(),
                                provider: provider_name.to_string(),
                            });
                        }

                        provider_name
                    }
                };

                Ok(serde_json::json!({
                    "name": name.to_case(Case::Snake),
                    "provider_name": provider_name.to_case(Case::Snake),
                    "name_upper": name.to_case(Case::ScreamingSnake),
                    "bucket": bucket.bucket,
                }))
            })
            .collect::<Result<Vec<_>, StorageConfigError>>()?;

        Ok(json!({
            "buckets": buckets,
            "configs": configs,
        }))
    }
}
