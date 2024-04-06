use std::str::FromStr;

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::subscriber::set_global_default;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{
    fmt::{format::FmtSpan, time::FormatTime, MakeWriter},
    layer::SubscriberExt,
    EnvFilter, Registry,
};

use crate::config::prefixed_env_var;

#[cfg(feature = "tracing_honeycomb")]
/// Configuration for sending telemetry to Honeycomb
#[derive(Deserialize, Debug, Clone)]
pub struct HoneycombConfig {
    /// The API key for this Honeycomb environment.  Sent in the `x-honeycomb-team` header
    pub api_key: String,
    /// The service name to use for this service.
    pub service_name: String,
    /// Override the default Honeycomb endpoint
    pub endpoint: Option<String>,
}

#[cfg(feature = "tracing_jaeger")]
/// Configuration for sending telemetry to Jaeger
#[derive(Deserialize, Debug, Clone)]
pub struct JaegerConfig {
    /// The Jaeger service name
    pub service_name: String,
    /// The Jaeger endpoint to send tracing to
    pub endpoint: String,
}

/// Configuration to define how to export telemetry
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(tag = "type")]
pub enum TracingExportConfig {
    /// Do not export tracing to an external service. This still prints it to the console.
    #[default]
    None,
    #[cfg(feature = "tracing_honeycomb")]
    /// Export traces to Honeycomb
    Honeycomb(HoneycombConfig),
    #[cfg(feature = "tracing_jaeger")]
    /// Export traces to Jaeger
    Jaeger(JaegerConfig),
}

/// Supported tracing providers
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TracingProvider {
    /// Do not export tracing to an external service
    #[default]
    None,
    /// Export traces to Honeycomb
    Honeycomb,
    /// Export traces to Jaeger
    Jaeger,
}

impl std::str::FromStr for TracingProvider {
    type Err = Report<TraceConfigureError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(TracingProvider::None),
            "honeycomb" => Ok(TracingProvider::Honeycomb),
            "jaeger" => Ok(TracingProvider::Jaeger),
            _ => Err(Report::new(TraceConfigureError))
                .attach_printable_lazy(|| format!("Unknown tracing provider: {s}")),
        }
    }
}

impl std::fmt::Display for TracingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TracingProvider::None => write!(f, "none"),
            TracingProvider::Honeycomb => write!(f, "honeycomb"),
            TracingProvider::Jaeger => write!(f, "jaeger"),
        }
    }
}

/// The error returned when tracing setup fails
#[derive(Error, Debug)]
#[error("Failed to configure tracing")]
pub struct TraceConfigureError;

/// Create a tracing configuration from the environment and some optional defaults
pub fn create_tracing_config(
    env_prefix: &str,
    default_type: TracingProvider,
    service_name: Option<String>,
    endpoint: Option<String>,
) -> Result<TracingExportConfig, Report<TraceConfigureError>> {
    let tracing_type = prefixed_env_var(env_prefix, "TRACING_TYPE")
        .ok()
        .map(|v| TracingProvider::from_str(&v))
        .transpose()?
        .unwrap_or(default_type);

    let config = match tracing_type {
        TracingProvider::None => TracingExportConfig::None,
        #[cfg(not(feature = "tracing_jaeger"))]
        TracingProvider::Jaeger => Err(Report::new(TraceConfigureError))
            .attach_printable("Jaeger tracing requires the `tracing_jaeger` feature")?,
        #[cfg(feature = "tracing_jaeger")]
        TracingProvider::Jaeger => TracingExportConfig::Jaeger(JaegerConfig {
            service_name: prefixed_env_var(env_prefix, "OTEL_SERVICE_NAME")
                .ok()
                .or(service_name)
                .unwrap_or_else(|| "api".to_string()),
            endpoint: prefixed_env_var(env_prefix, "OTEL_EXPORTER_OTLP_ENDPOINT")
                .change_context(TraceConfigureError)
                .attach_printable_lazy(|| {
                    format!(
                        "{env_prefix}OTEL_EXPORTER_OTLP_ENDPOINT must be set for Jaeger tracing"
                    )
                })?,
        }),
        #[cfg(not(feature = "tracing_honeycomb"))]
        TracingProvider::Honeycomb => Err(Report::new(TraceConfigureError))
            .attach_printable("Honeycomb tracing requires the `tracing_honeycomb` feature")?,
        #[cfg(feature = "tracing_honeycomb")]
        TracingProvider::Honeycomb => TracingExportConfig::Honeycomb(HoneycombConfig {
            api_key: prefixed_env_var(env_prefix, "HONEYCOMB_API_KEY")
                .change_context(TraceConfigureError)
                .attach_printable_lazy(|| {
                    format!("{env_prefix}HONEYCOMB_API_KEY must be set for Honeycomb tracing")
                })?,
            endpoint: endpoint.or_else(|| prefixed_env_var(env_prefix, "HONEYCOMB_ENDPOINT").ok()),
            service_name: prefixed_env_var(env_prefix, "OTEL_SERVICE_NAME")
                .ok()
                .or(service_name)
                .unwrap_or_else(|| "api".to_string()),
        }),
    };

    Ok(config)
}

/// Set up tracing, optionally exporting to an external service
pub fn configure_tracing<FT, W>(
    env_prefix: &str,
    export_config: TracingExportConfig,
    timer: FT,
    writer: W,
) -> Result<(), Report<TraceConfigureError>>
where
    FT: FormatTime + Send + Sync + 'static,
    for<'writer> W: MakeWriter<'writer> + Send + Sync + 'static,
{
    LogTracer::builder()
        .ignore_all([
            "rustls",
            // These spam the logs when debug level is turned on and we do CSS inlining on an email
            "selectors",
            "html5ever",
        ])
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_name = if env_prefix.is_empty() {
        "LOG".to_string()
    } else {
        format!("{}LOG", env_prefix)
    };

    let env_filter = EnvFilter::try_from_env(&env_name).unwrap_or_else(|_| EnvFilter::new("info"));

    let formatter = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::NEW)
        .with_timer(timer)
        .with_target(true)
        .with_writer(writer);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(formatter)
        .with(ErrorLayer::default());

    match export_config {
        #[cfg(feature = "tracing_honeycomb")]
        TracingExportConfig::Honeycomb(honeycomb_config) => {
            use opentelemetry_otlp::WithExportConfig;
            use tonic::metadata::{Ascii, MetadataValue};
            let mut otlp_meta = tonic::metadata::MetadataMap::new();
            otlp_meta.insert(
                "x-honeycomb-team",
                honeycomb_config
                    .api_key
                    .parse::<MetadataValue<Ascii>>()
                    .change_context(TraceConfigureError)?,
            );

            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(
                    honeycomb_config
                        .endpoint
                        .as_deref()
                        .unwrap_or("api.honeycomb.io:443"),
                )
                .with_metadata(otlp_meta);

            let otlp = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
                    opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                        "service.name",
                        honeycomb_config.service_name,
                    )]),
                ))
                .with_exporter(exporter)
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .change_context(TraceConfigureError)?;
            let telemetry = tracing_opentelemetry::layer().with_tracer(otlp);

            let subscriber = subscriber.with(telemetry);
            set_global_default(subscriber).expect("Setting subscriber");
        }
        #[cfg(feature = "tracing_jaeger")]
        TracingExportConfig::Jaeger(config) => {
            let tracer = opentelemetry_jaeger::new_agent_pipeline()
                .with_service_name(&config.service_name)
                .with_endpoint(config.endpoint.as_str())
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .unwrap();
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            let subscriber = subscriber.with(telemetry);
            set_global_default(subscriber).expect("Setting subscriber");
        }
        TracingExportConfig::None => {
            set_global_default(subscriber).expect("Setting subscriber");
        }
    };

    Ok(())
}

/// Shutdown tracing, and wait for remaining traces to be exported
pub async fn teardown_tracing() -> Result<(), tokio::task::JoinError> {
    #[cfg(feature = "opentelemetry")]
    tokio::task::spawn_blocking(|| {
        opentelemetry::global::shutdown_tracer_provider();
    })
    .await?;

    Ok(())
}

/// Initiailize tracing from a test context
pub mod test {
    use std::sync::Once;

    use tracing_subscriber::fmt::TestWriter;

    use super::TracingExportConfig;

    static TRACING: Once = Once::new();

    /// Initiialize tracing. This only starts tracing once per process, so is safe to
    /// call from every test.
    pub fn init() {
        TRACING.call_once(|| {
            super::configure_tracing(
                "TEST_",
                TracingExportConfig::None,
                tracing_subscriber::fmt::time::Uptime::default(),
                TestWriter::new(),
            )
            .expect("starting tracing");
        })
    }
}
