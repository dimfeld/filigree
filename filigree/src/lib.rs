#![warn(missing_docs)]
//! The non-generated components of the Filigree web framework

use std::borrow::Cow;

use serde::Serialize;

/// Authentication and Authorization
pub mod auth;
/// Error handling
pub mod errors;
/// A UUIDv7-based type for handling object IDs with a more compact representation.
pub mod object_id;
/// Utilities for running an HTTP server
pub mod server;
/// Utilities for working with SQL queries
pub mod sql;
#[cfg(test)]
pub mod testing;
/// Tracing configuration
#[cfg(feature = "tracing")]
pub mod tracing_config;

/// A simple structure for sending back a message-only response
#[derive(Serialize, Debug)]
pub struct Message<'a> {
    message: Cow<'a, str>,
}

impl<'a> Message<'a> {
    pub fn new(message: impl Into<Cow<'a, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
