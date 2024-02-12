use error_stack::{Report, ResultExt};
use thiserror::Error;
use tracing::subscriber::set_global_default;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{
    fmt::{format::FmtSpan, time::FormatTime, MakeWriter},
    layer::SubscriberExt,
    EnvFilter, Registry,
};

/// Configuration for sending telemetry to Honeycomb
pub struct HoneycombConfig {
    /// The Honeycomb team to export to
    pub team: String,
    /// The Honeycomb dataset to export to. This is also used as the service name
    pub dataset: String,
}

/// Configuration for sending telemetry to Jaeger
pub struct JaegerConfig {
    /// The Jaeger service name
    pub service_name: String,
    /// The Jaeger endpoint to send tracing to
    pub endpoint: String,
}

/// Configuration to define how to export telemetry
pub enum TracingExportConfig {
    /// Do not export tracing to an external service. This still prints it to the console.
    None,
    #[cfg(feature = "tracing_honeycomb")]
    /// Export traces to Honeycomb
    Honeycomb(HoneycombConfig),
    #[cfg(feature = "tracing_jaeger")]
    /// Export traces to Jaeger
    Jaeger(JaegerConfig),
}

/// The error returned when tracing setup fails
#[derive(Error, Debug)]
#[error("Failed to configure tracing")]
pub struct TraceConfigureError;

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
                    .team
                    .parse::<MetadataValue<Ascii>>()
                    .change_context(TraceConfigureError)?,
            );

            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("api.honeycomb.io:443")
                .with_metadata(otlp_meta);

            let otlp = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
                    opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                        "service.name",
                        honeycomb_config.dataset,
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
