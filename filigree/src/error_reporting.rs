//! Interface to error reporting services

#![cfg_attr(not(feature = "sentry"), allow(unused_variables))]

#[cfg(feature = "sentry")]
/// [Sentry](https://sentry.io) Error Reporting
pub mod sentry;

use std::fmt::Debug;

use error_stack::{Context, Report};
use serde::Serialize;

use crate::errors::HttpError;

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

/// An extension trait to report errors on any [HttpError]
pub trait HandleError {
    /// If this is an error, trace it and report it to the error reporting service
    fn report_error(self) -> Self;
    /// If this is an error, trace it and report it to the error reporting service with additional metadata
    fn report_error_with_info<META: Debug + Serialize + Send + Sync>(self, meta: &META) -> Self;
    /// Retun the error's [StatusCode] if it is an error, or [StatusCode::OK] otherwise
    fn status_code(&self) -> hyper::StatusCode;
}

impl<T, E> HandleError for Result<T, E>
where
    E: HttpError + std::error::Error + Sync + Send,
{
    fn report_error(self) -> Self {
        self.report_error_with_info(&())
    }

    fn report_error_with_info<META: Debug + Serialize + Send + Sync>(self, meta: &META) -> Self {
        if let Err(error) = &self {
            tracing::error!(?error, ?meta);

            let code = error.status_code();
            if code.is_server_error() {
                #[cfg(feature = "sentry")]
                sentry::Sentry::send_error_with_metadata(&error, meta);
            }
        }

        self
    }

    fn status_code(&self) -> hyper::StatusCode {
        match self {
            Ok(_) => hyper::StatusCode::OK,
            Err(error) => error.status_code(),
        }
    }
}

/// An extension trait to report errors on a [Report]. This is similar to [HandleErrorExt]
/// but needs to be a separate trait until specialization is stablized.
pub trait HandleErrorReport {
    /// If this is an error, trace it and report it to the error reporting service
    fn report_error(self) -> Self;
    /// If this is an error, trace it and report it to the error reporting service with additional metadata
    fn report_error_with_info<META: Debug + Serialize + Send + Sync>(self, meta: &META) -> Self;
    /// Retun the error's [StatusCode] if it is an error, or [StatusCode::OK] otherwise
    fn status_code(&self) -> hyper::StatusCode;
}

impl<T, E> HandleErrorReport for Result<T, error_stack::Report<E>>
where
    E: HttpError + std::error::Error + Sync + Send + 'static,
{
    fn report_error(self) -> Self {
        self.report_error_with_info(&())
    }

    fn report_error_with_info<META: Debug + Serialize + Send + Sync>(self, meta: &META) -> Self {
        if let Err(error) = &self {
            tracing::error!(error = ?error, ?meta);

            let code = error.current_context().status_code();
            if code.is_server_error() {
                #[cfg(feature = "sentry")]
                sentry::Sentry::send_report_with_metadata(error, meta);
            }
        }
        self
    }

    fn status_code(&self) -> hyper::StatusCode {
        match self {
            Ok(_) => hyper::StatusCode::OK,
            Err(error) => error.current_context().status_code(),
        }
    }
}
