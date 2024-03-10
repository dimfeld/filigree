use serde::{Deserialize, Serialize};
use url::Url;

use super::{local, s3, StorageError};

/// Special jurisdiction settings for R2 buckets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum R2Jurisdiction {
    /// EU Jurisdiction
    EU,
    /// FedRamp
    FedRamp,
}

impl R2Jurisdiction {
    fn url_segment(&self) -> &str {
        match self {
            Self::EU => "eu",
            Self::FedRamp => "fedramp",
        }
    }
}

/// Known storage providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "name")]
pub enum StorageProvider {
    /// AWS S3
    S3 {
        /// The AWS region
        region: Option<String>,
    },
    /// Digital Ocean Spaces
    DigitalOceanSpaces {
        /// The DO region
        region: String,
    },
    /// Backblaze B2
    BackblazeB2 {
        /// The B2 region
        region: String,
    },
    /// Cloudflare R2
    CloudflareR2 {
        /// The Cloudflare account ID
        account_id: String,
        /// The jurisdiction for the bucket that will be accessed via this provider, if any.
        jurisdiction: Option<R2Jurisdiction>,
    },
    /// Filesystem Storage
    Local {
        /// The base path where files should be stored
        path: Option<String>,
    },
}

impl StorageProvider {
    /// The prefix to use when figuring out provider-level environment variables.
    pub fn env_prefix(&self) -> &'static str {
        match self {
            Self::S3 { .. } => "S3",
            Self::DigitalOceanSpaces { .. } => "DO",
            Self::BackblazeB2 { .. } => "B2",
            Self::CloudflareR2 { .. } => "R2",
            Self::Local { .. } => "LOCAL",
        }
    }

    /// Generate a [StorageConfig] for this provider
    pub fn into_config(self) -> StorageConfig {
        match self {
            Self::S3 { region } => StorageConfig::S3(s3::S3StoreConfig {
                region,
                ..Default::default()
            }),
            Self::DigitalOceanSpaces { region } => StorageConfig::S3(s3::S3StoreConfig {
                endpoint: Some(
                    Url::parse(&format!("https://{region}.digitaloceanspaces.com")).unwrap(),
                ),
                region: Some(region),
                virtual_host_style: Some(true),
                ..Default::default()
            }),
            Self::BackblazeB2 { region } => StorageConfig::S3(s3::S3StoreConfig {
                endpoint: Some(
                    Url::parse(&format!("https://s3.{region}.backblazeb2.com")).unwrap(),
                ),
                region: Some(region),
                virtual_host_style: Some(true),
                ..Default::default()
            }),
            Self::CloudflareR2 {
                account_id,
                jurisdiction,
            } => {
                let endpoint = match jurisdiction {
                    Some(jurisdiction) => {
                        format!(
                            "https://{account_id}.{j}.r2.cloudflarestorage.com",
                            j = jurisdiction.url_segment()
                        )
                    }
                    None => format!("https://{account_id}.r2.cloudflarestorage.com"),
                };

                StorageConfig::S3(s3::S3StoreConfig {
                    endpoint: Some(Url::parse(&endpoint).unwrap()),
                    region: Some("auto".to_string()),
                    virtual_host_style: Some(false),
                    ..Default::default()
                })
            }
            Self::Local { path } => {
                StorageConfig::Local(local::LocalStoreConfig { base_path: path })
            }
        }
    }
}

/// Configuration for [Storage]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageConfig {
    #[cfg(feature = "storage_aws")]
    /// S3-compatible storage configuration
    S3(s3::S3StoreConfig),
    /// Local filesystem storage
    Local(local::LocalStoreConfig),
    /// In-memory storage for testing
    Memory,
}

impl StorageConfig {
    /// Create a [StorageConfig] from environment variables, potentially using the default
    /// settings.
    pub fn from_env(
        default_settings: StorageConfig,
        storage_prefix: &str,
        provider_prefix: &str,
    ) -> Result<Self, StorageError> {
        let mut settings =
            if let Ok(provider_type) = std::env::var(format!("{storage_prefix}_PROVIDER_TYPE")) {
                // The provider type may be overridden, so create a new settings object if it has
                // been changed from the default.
                match provider_type.to_uppercase().as_str() {
                    #[cfg(feature = "storage_aws")]
                    "S3" => match default_settings {
                        StorageConfig::S3(_) => default_settings,
                        _ => StorageConfig::S3(s3::S3StoreConfig::default()),
                    },
                    "LOCAL" => match default_settings {
                        StorageConfig::Local(_) => default_settings,
                        _ => StorageConfig::Local(local::LocalStoreConfig::default()),
                    },
                    _ => Err(StorageError::Configuration(
                        "Unknown storage setting in PROVIDER_TYPE",
                    ))?,
                }
            } else {
                default_settings
            };

        settings.merge_env(provider_prefix)?;
        settings.merge_env(storage_prefix)?;

        Ok(settings)
    }

    /// Merge environment variables into a [StorageConfig]
    pub fn merge_env(&mut self, prefix: &str) -> Result<(), StorageError> {
        match self {
            #[cfg(feature = "storage_aws")]
            StorageConfig::S3(options) => options.merge_env(prefix)?,
            StorageConfig::Local(options) => options.merge_env(prefix)?,
            StorageConfig::Memory => {}
        }
        Ok(())
    }
}
