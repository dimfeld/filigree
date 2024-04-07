#[cfg(feature = "sentry")]
// [Sentry](https://sentry.io) Error Reporting
pub mod sentry;

use error_stack::Context;
use serde::Serialize;
use tracing::Level;

#[async_trait::async_trait]
pub trait ErrorReporter {
    /// Send an error to the error reporting service
    async fn send_error<E: std::error::Error + Send>(&self, err: E) {
        self.send_error_with_metadata(err, ()).await
    }

    /// Send an [error_stack::Report] to the error reporting service
    async fn send_report<C: Context>(&self, err: &error_stack::Report<C>) {
        self.send_report_with_metadata(err, ()).await
    }

    /// Send an error with additional metadata.
    async fn send_error_with_metadata<E: std::error::Error + Send, T: Serialize + Send>(
        &self,
        err: E,
        metadata: T,
    );

    /// Send an [error_stack::Report] with additional metadata.
    async fn send_report_with_metadata<C: Context, T: Serialize + Send>(
        &self,
        err: &error_stack::Report<C>,
        metadata: T,
    );

    /// Send a plain message to the error reporting service
    async fn send_message(&self, level: Level, message: &str);
}
