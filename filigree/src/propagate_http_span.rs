//! Helpers to propagate HTTP spans to another server

use opentelemetry::{
    propagation::{Extractor, Injector, TextMapPropagator},
    Context,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Extend [reqwest::RequestBuilder] with a function to propagate tracing span information
pub trait PropagateSpan {
    /// Inject the current [Span] context into the HTTP headers
    fn propagate_span(self) -> reqwest::RequestBuilder;
}

impl PropagateSpan for reqwest::RequestBuilder {
    fn propagate_span(self) -> reqwest::RequestBuilder {
        HeaderInjector {
            builder: Some(self),
        }
        .inject()
    }
}

struct HeaderInjector {
    builder: Option<reqwest::RequestBuilder>,
}

impl HeaderInjector {
    fn inject(mut self) -> reqwest::RequestBuilder {
        let span = tracing::Span::current();
        let context = span.context();
        let propagator = TraceContextPropagator::new();
        propagator.inject_context(&context, &mut self);

        self.builder.unwrap()
    }
}

impl Injector for HeaderInjector {
    fn set(&mut self, key: &str, value: String) {
        // `header` consumes the builder and returns a new one, so we move the builder out of the
        // option and then back into it again.
        let b = self.builder.take().unwrap().header(key, value);
        self.builder = Some(b);
    }
}

struct HeaderExtractor<'a> {
    req: &'a axum::extract::Request,
}

impl<'a> HeaderExtractor<'a> {
    fn extract(&self) -> Context {
        let propagator = TraceContextPropagator::new();
        let context = propagator.extract(self);
        context
    }
}

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.req.headers().get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.req.headers().keys().map(|s| s.as_str()).collect()
    }
}

/// Extract parent span inforation from the HTTP request
/// The returned context can be set on the current span using
/// `tracing::Span::current().set_parent(context)`.
pub fn extract_request_parent(req: &axum::extract::Request) -> Context {
    let extractor = HeaderExtractor { req };
    extractor.extract()
}
