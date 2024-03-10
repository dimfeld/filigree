#![warn(missing_docs)]
//! The non-generated components of the Filigree web framework

use std::{borrow::Cow, env::VarError};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Authentication and Authorization
pub mod auth;
/// Email templates and sending
pub mod email;
/// Error handling
pub mod errors;
/// Request extractors
pub mod extract;
/// Extension trait to inspect the body of a Reqwest error response
pub mod inspect_response;
/// A UUIDv7-based type for handling object IDs with a more compact representation.
pub mod object_id;
/// Common query string structures for use with [axum::extract::Query]
pub mod query_string;
/// Request Handling and Validation
pub mod requests;
/// Utilities for running an HTTP server
pub mod server;
/// Utilities for working with SQL queries
pub mod sql;
/// Cloud object storage
pub mod storage;
/// Functionality to help test your app
pub mod testing;
/// Tracing configuration
#[cfg(feature = "tracing")]
pub mod tracing_config;
/// Manage users, roles, and related data
pub mod users;

/// A simple structure for sending back a message-only response
#[derive(Serialize, Debug)]
pub struct Message<'a> {
    message: Cow<'a, str>,
}

impl<'a> Message<'a> {
    /// Create a new Message with the given text.
    pub fn new(message: impl Into<Cow<'a, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// A request body that only contains an email
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EmailBody {
    /// The email address
    #[validate(email)]
    pub email: String,
}

/// Get an environment variable with an optional prefix
pub fn prefixed_env_var(prefix: &str, key: &str) -> Result<String, VarError> {
    if prefix.is_empty() {
        std::env::var(key)
    } else {
        std::env::var(format!("{prefix}{key}"))
    }
}

/// Parse an `Option<String>`, returning an error if the value is present and fails to parse to the expected type.
pub fn parse_option<T: std::str::FromStr>(value: Option<String>) -> Result<Option<T>, T::Err> {
    match value {
        Some(v) => Ok(Some(v.parse()?)),
        None => Ok(None),
    }
}

/// Try two options for an environment variable, or fallback to a default value. This is used
/// when setting up storage providers.
pub fn double_env_var_fallback<T: std::str::FromStr>(
    first: &str,
    second: &str,
    fallback_value: T,
) -> Result<T, T::Err> {
    if let Ok(val) = std::env::var(first) {
        val.parse()
    } else if let Ok(val) = std::env::var(second) {
        val.parse()
    } else {
        Ok(fallback_value)
    }
}
