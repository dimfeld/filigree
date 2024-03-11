//! S3 storage configuration for object_store
use object_store::aws::AmazonS3;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use url::Url;

use super::StorageError;
use crate::config::{merge_option_if_set, parse_option, prefixed_env_var};

/// Configuration for an S3 store
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct S3StoreConfig {
    /// The endpoint to use when connecting to the service, if not the default AWS S3 endpoint.
    pub endpoint: Option<Url>,
    /// The region to connect to
    pub region: Option<String>,
    /// The access key id to authenticate with
    pub access_key_id: Option<String>,
    /// The secret key to authenticate with
    pub secret_key: Option<String>,
    /// If this service requires connecting with "virtual host" style, in which the bucket name is
    /// part of the URL.
    #[serde(default)]
    pub virtual_host_style: Option<bool>,
    /// If true, configure the client for an AWS S3 Express zone bucket
    pub s3_express: Option<bool>,
}

impl S3StoreConfig {
    /// Overwrite this configuration's values with environment values, if set.
    pub fn merge_env(&mut self, prefix: &str) -> Result<(), StorageError> {
        let from_env = S3StoreConfig::from_env(prefix)?;
        merge_option_if_set(&mut self.endpoint, from_env.endpoint);
        merge_option_if_set(&mut self.region, from_env.region);
        merge_option_if_set(&mut self.access_key_id, from_env.access_key_id);
        merge_option_if_set(&mut self.secret_key, from_env.secret_key);
        merge_option_if_set(&mut self.virtual_host_style, from_env.virtual_host_style);
        merge_option_if_set(&mut self.s3_express, from_env.s3_express);

        Ok(())
    }

    /// Create a new S3StoreConfig from environment variables
    pub fn from_env(prefix: &str) -> Result<Self, StorageError> {
        let config = S3StoreConfig {
            endpoint: parse_option(prefixed_env_var(prefix, "S3_ENDPOINT").ok())
                .map_err(|_| StorageError::Configuration("S3 endpoint must be a URI"))?,
            region: prefixed_env_var(prefix, "S3_REGION").ok(),
            access_key_id: prefixed_env_var(prefix, "S3_ACCESS_KEY_ID").ok(),
            secret_key: prefixed_env_var(prefix, "S3_SECRET_ACCESS_KEY").ok(),
            s3_express: parse_option(prefixed_env_var(prefix, "S3_EXPRESS").ok())
                .map_err(|_| StorageError::Configuration("S3_EXPRESS must be true or false"))?,
            virtual_host_style: parse_option(
                prefixed_env_var(prefix, "S3_VIRTUAL_HOST_STYLE").ok(),
            )
            .map_err(|_| {
                StorageError::Configuration("S3_VIRTUAL_HOST_STYLE must be true or false")
            })?,
        };

        match (config.access_key_id.is_some(), config.secret_key.is_some()) {
            (true, true) => Ok(config),
            (false, false) => Ok(config),
            _ => Err(StorageError::Configuration(
                "Must provide both or none of access_key_id and secret_key",
            )),
        }
    }

    #[cfg(feature = "filigree-cli")]
    /// Recreate the structure in Rust code.
    pub fn template_text(&self) -> String {
        use crate::templates::{OptionAsStorageUrl, OptionAsString};

        format!(
            "StorageConfig::S3(filigree::storage::s3::S3StoreConfig {{
                endpoint: {},
                region: {},
                access_key_id: {},
                secret_key: {},
                virtual_host_style: {:?},
                s3_express: {:?},
            }})",
            OptionAsStorageUrl(&self.endpoint),
            OptionAsString(&self.region),
            OptionAsString(&self.access_key_id),
            OptionAsString(&self.secret_key),
            self.virtual_host_style,
            self.s3_express
        )
    }
}

/// Create a new S3 store. This function is mostly designed to make it easier to use S3-compatible
/// services from other providers. For real S3, it may be simpler to just use
/// [AmazonS3Builder::from_env()] or a similar function.
pub fn create_store<'a>(config: &S3StoreConfig, bucket: &'a str) -> Result<AmazonS3, StorageError> {
    let virtual_host_style = config.virtual_host_style.unwrap_or(false);
    let mut builder = object_store::aws::AmazonS3Builder::new()
        .with_virtual_hosted_style_request(virtual_host_style)
        .with_bucket_name(bucket)
        .with_s3_express(config.s3_express.unwrap_or(false));

    match (config.access_key_id.as_ref(), config.secret_key.as_ref()) {
        (Some(access_key_id), Some(secret_key)) => {
            builder = builder
                .with_access_key_id(access_key_id.as_str())
                .with_secret_access_key(secret_key.as_str());
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(StorageError::Configuration(
                "Must provide both or none of access_key_id and secret_key",
            ));
        }
        (None, None) => {}
    };

    if let Some(endpoint) = config.endpoint.as_ref() {
        event!(Level::TRACE, ?endpoint);

        // When using virtual host style, object_store requires us to prepend the bucket name
        // to the endpoint.
        let host = endpoint.host_str().ok_or(StorageError::Configuration(
            "Missing host in S3 endpoint URL",
        ))?;

        let e = if virtual_host_style && !host.starts_with(bucket) {
            let mut endpoint = endpoint.clone();
            let new_domain = format!("{bucket}.{host}");
            endpoint.set_host(Some(&new_domain)).map_err(|_| {
                StorageError::Configuration("Unable to construct virtual host S3 Endpoint URL")
            })?;

            endpoint.to_string()
        } else {
            endpoint.to_string()
        };
        event!(Level::DEBUG, endpoint=%e, "Creating S3 provider with custom endpoint");
        builder = builder.with_endpoint(e);
    }

    if let Some(region) = config.region.as_ref() {
        builder = builder.with_region(region.as_str());
    }

    let store = builder.build()?;
    Ok(store)
}
