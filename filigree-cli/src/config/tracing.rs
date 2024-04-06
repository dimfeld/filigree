use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(tag = "type")]
pub struct TracingConfig {
    pub provider: filigree::tracing_config::TracingProvider,

    /// The service name for the API service. If omitted, `api` is used.
    #[serde(default = "default_api_service_name")]
    pub api_service_name: String,
    /// The endpoint to send traces to. This can be omitted for Honeycomb but is
    /// required to be specified here or in the environment for Jaeger.
    pub endpoint: Option<String>,
}

fn default_api_service_name() -> String {
    String::from("api")
}
