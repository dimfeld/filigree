//! S3 storage configuration for object_store
use http::{
    uri::{Authority, Scheme},
    Uri,
};
use object_store::aws::AmazonS3;
use tracing::{event, Level};

use super::StorageError;

/// Configuration for an S3 store
#[derive(Debug, Default, Clone)]
pub struct S3StoreConfig {
    /// The endpoint to use when connecting to the service, if not the default AWS S3 endpoint.
    pub endpoint: Option<Uri>,
    /// The region to connect to
    pub region: Option<String>,
    /// The access key id to authenticate with
    pub access_key_id: Option<String>,
    /// The secret key to authenticate with
    pub secret_key: Option<String>,
    /// If this service requires connecting with "virtual host" style, in which the bucket name is
    /// part of the URL.
    pub virtual_host_style: bool,
}

/// Create a new S3 store. This function is mostly designed to make it easier to use S3-compatible
/// services from other providers. For real S3, it may be simpler to just use
/// [AmazonS3Builder::from_env()] or a similar function.
pub fn create_store<'a>(config: &S3StoreConfig, bucket: &'a str) -> Result<AmazonS3, StorageError> {
    let mut builder = object_store::aws::AmazonS3Builder::new()
        .with_virtual_hosted_style_request(config.virtual_host_style)
        .with_bucket_name(bucket);

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
        event!(Level::DEBUG, ?endpoint);
        let needs_scheme = endpoint.scheme().is_none();

        let e = if config.virtual_host_style {
            // When using virtual host style, object_store requires us to prepend the bucket name
            // to the endpoint.
            let parts = endpoint.to_owned().into_parts();
            let authority = parts
                .authority
                .unwrap_or_else(|| Authority::from_static("missing-host"));
            let new_domain = format!("{}.{}", bucket, authority.as_str());
            let scheme = parts.scheme.unwrap_or(Scheme::HTTPS);

            format!("{}://{}", scheme.as_str(), new_domain)
        } else if needs_scheme {
            // We tolerate a missing https:// in the endpoint, but object_store will panic without it.
            let parts = endpoint.to_owned().into_parts();
            format!("https://{}", parts.authority.unwrap().as_str())
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
