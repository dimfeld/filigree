//! Interface to error reporting services

#![cfg_attr(not(feature = "sentry"), allow(unused_variables))]

#[cfg(feature = "sentry")]
// [Sentry](https://sentry.io) Error Reporting
pub mod sentry;

use error_stack::{Context, Report};
use serde::Serialize;

/// An error reporting service
#[derive(Default)]
pub enum ErrorReporter {
    #[cfg(feature = "sentry")]
    /// sentry.io
    Sentry,
    /// An error reporter that does nothing, for cases where no service is enabled.
    #[default]
    Noop,
}

impl ErrorReporter {
    /// Send an [std::error::Error]
    pub async fn send_error<E: std::error::Error + Send>(&self, err: &E) {
        match self {
            #[cfg(feature = "sentry")]
            ErrorReporter::Sentry => sentry::Sentry::send_error(err),
            ErrorReporter::Noop => {}
        }
    }

    /// Send an [error_stack::Report]
    pub async fn send_report<C: Context>(&self, err: &Report<C>) {
        match self {
            #[cfg(feature = "sentry")]
            ErrorReporter::Sentry => sentry::Sentry::send_report(err),
            ErrorReporter::Noop => {}
        }
    }

    /// Send an [std::error::Error] with additional metadata.
    pub async fn send_error_with_metadata<E: std::error::Error + Send, T: Serialize + Send>(
        &self,
        err: &E,
        metadata: &T,
    ) {
        match self {
            #[cfg(feature = "sentry")]
            ErrorReporter::Sentry => sentry::Sentry::send_error_with_metadata(err, metadata),
            ErrorReporter::Noop => {}
        }
    }

    /// Send an [error_stack::Report] with additional metadata.
    pub async fn send_report_with_metadata<C: Context, T: Serialize + Send>(
        &self,
        err: &error_stack::Report<C>,
        metadata: &T,
    ) {
        match self {
            #[cfg(feature = "sentry")]
            ErrorReporter::Sentry => sentry::Sentry::send_report_with_metadata(err, metadata),
            ErrorReporter::Noop => {}
        }
    }

    /// Send a text message
    pub async fn send_message(&self, level: tracing::Level, message: &str) {
        match self {
            #[cfg(feature = "sentry")]
            ErrorReporter::Sentry => sentry::Sentry::send_message(level, message),
            ErrorReporter::Noop => {}
        }
    }
}
