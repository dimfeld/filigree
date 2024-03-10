use serde::{Deserialize, Serialize};
use url::Url;

use super::{local, s3, StorageError};
use crate::config::{merge_option_if_set, parse_option, prefixed_env_var};

/// Special jurisdiction settings for R2 buckets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum R2Jurisdiction {
    /// EU Jurisdiction
    EU,
    /// FedRamp
    FedRamp,
}

impl std::str::FromStr for R2Jurisdiction {
    type Err = StorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eu" => Ok(Self::EU),
            "fedramp" => Ok(Self::FedRamp),
            _ => Err(StorageError::Configuration("Unknown R2 jurisdiction")),
        }
    }
}

impl R2Jurisdiction {
    fn url_segment(&self) -> &str {
        match self {
            Self::EU => "eu",
            Self::FedRamp => "fedramp",
        }
    }
}

/// Known storage providers. Values such as region and R2 account ID are usually required,
/// but left as Options here to facilitate merging envirionment variables into fixed defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "preset")]
pub enum StorageProvider {
    /// AWS S3
    S3 {
        /// The AWS region
        region: Option<String>,
    },
    /// Digital Ocean Spaces
    DigitalOceanSpaces {
        /// The DO region
        region: Option<String>,
    },
    /// Backblaze B2
    BackblazeB2 {
        /// The B2 region
        region: Option<String>,
    },
    /// Cloudflare R2
    CloudflareR2 {
        /// The Cloudflare account ID. This should usually be set from the environment
        /// as it may be considered sensitive information.
        account_id: Option<String>,
        /// The jurisdiction for the bucket that will be accessed via this provider, if any.
        jurisdiction: Option<R2Jurisdiction>,
    },
}

impl StorageProvider {
    #[cfg(feature = "filigree-cli")]
    /// Recreate the structure in Rust code.
    pub fn template_text(&self) -> String {
        match self {
            StorageProvider::S3 { region } => {
                format!("StorageProvider::S3 {{ region: {:?} }}", region)
            }
            StorageProvider::DigitalOceanSpaces { region } => {
                format!(
                    "StorageProvider::DigitalOceanSpaces {{ region: {:?} }}",
                    region
                )
            }
            StorageProvider::BackblazeB2 { region } => {
                format!("StorageProvider::BackblazeB2 {{ region: {:?} }}", region)
            }
            StorageProvider::CloudflareR2 {
                account_id,
                jurisdiction,
            } => format!(
                "StorageProvider::CloudflareR2 {{ account_id: {:?}, jurisdiction: {:?} }}",
                account_id, jurisdiction
            ),
        }
    }

    /// Merge environment variables into this provider
    pub fn merge_env(&mut self, prefix: &str) -> Result<(), StorageError> {
        match self {
            StorageProvider::S3 { region } => {
                merge_option_if_set(region, prefixed_env_var(prefix, "REGION").ok());
            }
            StorageProvider::DigitalOceanSpaces { region } => {
                merge_option_if_set(region, prefixed_env_var(prefix, "REGION").ok());
            }
            StorageProvider::BackblazeB2 { region } => {
                merge_option_if_set(region, prefixed_env_var(prefix, "REGION").ok());
            }
            StorageProvider::CloudflareR2 {
                account_id,
                jurisdiction,
            } => {
                merge_option_if_set(account_id, prefixed_env_var(prefix, "ACCOUNT_ID").ok());
                merge_option_if_set(
                    jurisdiction,
                    parse_option(prefixed_env_var(prefix, "JURISDICTION").ok())?,
                );
            }
        };

        Ok(())
    }

    /// Generate a [StorageConfig] for this provider
    pub fn into_config(self) -> Result<StorageConfig, StorageError> {
        let config = match self {
            Self::S3 { region } => StorageConfig::S3(s3::S3StoreConfig {
                region,
                ..Default::default()
            }),
            Self::DigitalOceanSpaces { region } => {
                let region = region.ok_or(StorageError::Configuration(
                    "Missing region in DigitalOcean Spaces config",
                ))?;

                StorageConfig::S3(s3::S3StoreConfig {
                    endpoint: Some(
                        Url::parse(&format!("https://{region}.digitaloceanspaces.com")).unwrap(),
                    ),
                    region: Some(region),
                    virtual_host_style: Some(true),
                    ..Default::default()
                })
            }
            Self::BackblazeB2 { region } => {
                let region = region.ok_or(StorageError::Configuration(
                    "Missing region in Backblaze B2 config",
                ))?;

                StorageConfig::S3(s3::S3StoreConfig {
                    endpoint: Some(
                        Url::parse(&format!("https://s3.{region}.backblazeb2.com")).unwrap(),
                    ),
                    region: Some(region),
                    virtual_host_style: Some(true),
                    ..Default::default()
                })
            }
            Self::CloudflareR2 {
                account_id,
                jurisdiction,
            } => {
                let account_id = account_id.ok_or(StorageError::Configuration(
                    "Missing account ID in Cloudflare R2 config",
                ))?;

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
        };

        Ok(config)
    }
}

/// Configuration for [Storage]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
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
        env_prefix: &str,
    ) -> Result<Self, StorageError> {
        let mut settings =
            if let Ok(provider_type) = std::env::var(format!("{env_prefix}PROVIDER_TYPE")) {
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

        settings.merge_env(env_prefix)?;

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

    #[cfg(feature = "filigree-cli")]
    /// Recreate the structure as Rust code for the CLI template
    pub fn template_text(&self) -> String {
        match self {
            Self::S3(config) => config.template_text(),
            Self::Local(config) => config.template_text(),
            Self::Memory => "StorageConfig::Memory".to_string(),
        }
    }
}
