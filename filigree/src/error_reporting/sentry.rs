use std::error::Error;

use error_stack::{Context, FrameKind, Report};
use sentry::{
    protocol::{Event, Exception},
    Hub,
};
use serde::Serialize;
use uuid::Uuid;

use super::ErrorReporter;

pub struct Sentry {}

impl Sentry {
    fn init() {}
}

#[async_trait::async_trait]
impl ErrorReporter for Sentry {
    async fn send_error<E: std::error::Error + Send>(&self, err: E) {
        sentry::capture_error(&err);
    }

    async fn send_report<C: Context>(&self, err: &Report<C>) {
        Hub::with_active(|hub| hub.capture_report(err));
    }

    async fn send_error_with_metadata<E: std::error::Error + Send, T: Serialize + Send>(
        &self,
        err: E,
        metadata: T,
    ) {
        sentry::with_scope(
            |scope| {
                if let Ok(val) = serde_json::to_value(metadata) {
                    scope.set_extra("metadata", val);
                }
            },
            || sentry::capture_error(&err),
        );
    }

    /// Send an [error_stack::Report] with additional metadata.
    async fn send_report_with_metadata<C: Context, T: Serialize + Send>(
        &self,
        err: &error_stack::Report<C>,
        metadata: T,
    ) {
        sentry::with_scope(
            |scope| {
                if let Ok(val) = serde_json::to_value(metadata) {
                    scope.set_extra("metadata", val);
                }
            },
            || Hub::with_active(|hub| hub.capture_report(err)),
        );
    }

    async fn send_message(&self, level: tracing::Level, message: &str) {
        let level = match level {
            tracing::Level::ERROR => sentry::Level::Error,
            tracing::Level::WARN => sentry::Level::Warning,
            tracing::Level::INFO => sentry::Level::Info,
            tracing::Level::DEBUG => sentry::Level::Debug,
            tracing::Level::TRACE => sentry::Level::Debug,
        };

        sentry::capture_message(message, level);
    }
}

/// Hub extension methods for working with [error_stack::Report].
pub trait ErrorStackHubExt {
    /// Send an [error_stack::Report] to Sentry
    fn capture_report(&self, report: &error_stack::Report<impl Context>) -> Uuid;
}

impl ErrorStackHubExt for Hub {
    fn capture_report(&self, report: &error_stack::Report<impl Context>) -> Uuid {
        let event = event_from_report(report);
        Hub::with_active(|hub| hub.capture_event(event))
    }
}

/// Create a Sentry [Event] from an [Report].
pub fn event_from_report(report: &error_stack::Report<impl Context>) -> Event<'static> {
    let main_err_dbg = format!("{:?}", report.current_context());

    // TODO Attach spantrace if there is one
    // TODO Attach backtrace if there is one
    // TODO This should walk the frames and make each one an exception.
    let ty = sentry::parse_type_from_debug(&main_err_dbg);
    let value = format!("{:?}", report);
    let exception = Exception {
        ty: ty.to_string(),
        value: Some(value),
        ..Default::default()
    };

    Event {
        event_id: Uuid::now_v7(),
        exception: vec![exception].into(),
        level: sentry::Level::Error,

        ..Default::default()
    }
}

// a list of frames is a singly-linked list where items are pushed onto the front,
// so whenever we see an attachment we know that it will be associated with the next
// Context that we see in the list

/*
fn exception_from_context(err: &dyn Context) -> Exception {
    let dbg = format!("{err:?}");
    let value = err.to_string();
    let ty = sentry::parse_type_from_debug(&dbg);

    Exception {
        ty: ty.to_string(),
        value: Some(value),
        ..Default::default()
    }
}
*/
