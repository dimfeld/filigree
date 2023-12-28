#![warn(missing_docs)]
//! The non-generated components of the Filigree web framework

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
/// Tracing configuration
#[cfg(feature = "tracing")]
pub mod tracing_config;
