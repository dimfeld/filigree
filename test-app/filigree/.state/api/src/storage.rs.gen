//! Object storage configuration

#![allow(unused_imports)]

use error_stack::{Report, ResultExt};
use filigree::{
    config::parse_option,
    storage::{Storage, StorageConfig, StorageError, StoragePreset},
};
use url::Url;

pub struct AppStorage {
    pub image_hosting: Storage,
    pub image_uploads: Storage,
    pub pdfs: Storage,
    pub config_cdn: StorageConfig,
    pub config_disk: StorageConfig,
}

impl AppStorage {
    pub fn new(config: AppStorageConfig) -> Result<AppStorage, Report<StorageError>> {
        Ok(AppStorage {
            image_hosting: Storage::new(&config.image_hosting.config, config.image_hosting.bucket)
                .attach_printable("Unable to create storage for image_hosting")?
                .with_public_url(config.image_hosting.public_url),
            image_uploads: Storage::new(&config.image_uploads.config, config.image_uploads.bucket)
                .attach_printable("Unable to create storage for image_uploads")?
                .with_public_url(config.image_uploads.public_url),
            pdfs: Storage::new(&config.pdfs.config, config.pdfs.bucket)
                .attach_printable("Unable to create storage for pdfs")?
                .with_public_url(config.pdfs.public_url),
            config_cdn: config.config_cdn,
            config_disk: config.config_disk,
        })
    }
}

pub struct AppStorageConfigEntry {
    pub config: StorageConfig,
    pub bucket: String,
    pub public_url: Option<Url>,
}

pub struct AppStorageConfig {
    pub image_hosting: AppStorageConfigEntry,
    pub image_uploads: AppStorageConfigEntry,
    pub pdfs: AppStorageConfigEntry,
    pub config_cdn: StorageConfig,
    pub config_disk: StorageConfig,
}

impl AppStorageConfig {
    /// Create the application storage configuration based on the filigree configuration files
    /// and environment variables.
    pub fn new() -> Result<AppStorageConfig, StorageError> {
        let config_cdn = StorageConfig::from_env(
            filigree::storage::StoragePreset::CloudflareR2 {
                account_id: Some(r##"define-in-env"##.to_string()),
                jurisdiction: None,
            }
            .into_config()?,
            "STORAGE_PROVIDER_CDN_",
        )?;

        let config_disk = StorageConfig::from_env(
            StorageConfig::Local(filigree::storage::local::LocalStoreConfig {
                base_path: Some(r##"/tmp/filigree-test-storage/internal"##.to_string()),
            }),
            "STORAGE_PROVIDER_DISK_",
        )?;

        let mut bucket_config_image_hosting = config_cdn.clone();
        bucket_config_image_hosting.merge_env("STORAGE_IMAGE_HOSTING_")?;

        let image_hosting_bucket = std::env::var("STORAGE_IMAGE_HOSTING_BUCKET")
            .unwrap_or_else(|_| "fl-test-image-input".to_string());

        let image_hosting_public_url: Option<Url> =
            parse_option(std::env::var("STORAGE_IMAGE_HOSTING_PUBLIC_URL").ok())
                .map_err(|_| {
                    StorageError::Configuration("Invalid URL in STORAGE_IMAGE_HOSTING_PUBLIC_URL")
                })?
                .or(Some(
                    Url::parse("https://cdn.example.com/").expect("URL from template was invalid"),
                ));

        let mut bucket_config_image_uploads = config_disk.clone();
        bucket_config_image_uploads.merge_env("STORAGE_IMAGE_UPLOADS_")?;

        let image_uploads_bucket = std::env::var("STORAGE_IMAGE_UPLOADS_BUCKET")
            .unwrap_or_else(|_| "fl-test-image-uploads".to_string());

        let image_uploads_public_url: Option<Url> =
            parse_option(std::env::var("STORAGE_IMAGE_UPLOADS_PUBLIC_URL").ok()).map_err(|_| {
                StorageError::Configuration("Invalid URL in STORAGE_IMAGE_UPLOADS_PUBLIC_URL")
            })?;

        let mut bucket_config_pdfs = config_disk.clone();
        bucket_config_pdfs.merge_env("STORAGE_PDFS_")?;

        let pdfs_bucket =
            std::env::var("STORAGE_PDFS_BUCKET").unwrap_or_else(|_| "fl-test-pdfs".to_string());

        let pdfs_public_url: Option<Url> =
            parse_option(std::env::var("STORAGE_PDFS_PUBLIC_URL").ok()).map_err(|_| {
                StorageError::Configuration("Invalid URL in STORAGE_PDFS_PUBLIC_URL")
            })?;

        Ok(AppStorageConfig {
            image_hosting: AppStorageConfigEntry {
                config: bucket_config_image_hosting,
                bucket: image_hosting_bucket,
                public_url: image_hosting_public_url,
            },
            image_uploads: AppStorageConfigEntry {
                config: bucket_config_image_uploads,
                bucket: image_uploads_bucket,
                public_url: image_uploads_public_url,
            },
            pdfs: AppStorageConfigEntry {
                config: bucket_config_pdfs,
                bucket: pdfs_bucket,
                public_url: pdfs_public_url,
            },
            config_cdn,
            config_disk,
        })
    }

    /// A test configuration that forces all storage providers to be in-memory.
    pub fn new_in_memory() -> AppStorageConfig {
        AppStorageConfig {
            image_hosting: AppStorageConfigEntry {
                config: StorageConfig::Memory,
                bucket: "fl-test-image-input".to_string(),
                public_url: Some(
                    Url::parse("https://cdn.example.com/").expect("URL from template was invalid"),
                ),
            },
            image_uploads: AppStorageConfigEntry {
                config: StorageConfig::Memory,
                bucket: "fl-test-image-uploads".to_string(),
                public_url: None,
            },
            pdfs: AppStorageConfigEntry {
                config: StorageConfig::Memory,
                bucket: "fl-test-pdfs".to_string(),
                public_url: None,
            },
            config_cdn: StorageConfig::Memory,
            config_disk: StorageConfig::Memory,
        }
    }
}
