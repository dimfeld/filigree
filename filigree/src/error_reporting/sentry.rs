//! Support for Sentry error reporting

use error_stack::{AttachmentKind, Context, Report};
use itertools::Itertools;
use sentry::{
    protocol::{Event, Exception},
    Hub,
};
use serde::Serialize;
use uuid::Uuid;

use super::ErrorReporter;
use crate::error_stack::{
    ContextWithAttachments, ContextWithAttachmentsExt, ErrorStackInformation,
};

/// Sentry error reporting
pub struct Sentry {}

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
    let info = ErrorStackInformation::new(report);
    let exceptions = report
        .frames()
        .by_error()
        .map(|e| {
            let result = exception_from_context_and_attachments(e);
            result
        })
        .collect::<Vec<_>>();

    let extra = [
        ("backtrace", info.backtrace.map(|b| b.to_string())),
        ("spantrace", info.spantrace.map(|s| s.to_string())),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|s| (k.to_string(), serde_json::Value::from(s))))
    .collect();

    Event {
        event_id: Uuid::now_v7(),
        exception: exceptions.into(),
        level: sentry::Level::Error,
        extra,
        ..Default::default()
    }
}

fn exception_from_context_and_attachments<'a>(err: ContextWithAttachments<'a>) -> Exception {
    let dbg = format!("{:?}", err.context);
    let attachments = err.attachments.iter().filter_map(|a| match a {
        AttachmentKind::Printable(p) => Some(format!("  {p}")),
        _ => None,
    });
    let value = std::iter::once(err.context.to_string())
        .chain(attachments)
        .join("\n");
    let ty = sentry::parse_type_from_debug(&dbg);

    Exception {
        ty: ty.to_string(),
        value: Some(value),
        ..Default::default()
    }
}
