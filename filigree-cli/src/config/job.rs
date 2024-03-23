use serde::{Deserialize, Serialize};

/// Configuration for a background job
#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct Job {
    /// The default priority for this job. Defaults to 1, and jobs with higher priority will be run first
    #[serde(default = "default_priority")]
    pub priority: i32,

    /// The default weight for this job. Defaults to 1, and this value counts against the
    /// concurrency limit for a worker.
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// How long to wait for this job to run before retrying or failing.
    pub timeout: Option<std::time::Duration>,

    /// Schedules to automatically run this job, if any
    #[serde(default)]
    pub schedule: Vec<JobSchedule>,
}

fn default_priority() -> i32 {
    1
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobSchedule {
    /// A cron string that specifies how this job will run
    schedule: String,

    /// A payload to invoke the job with
    payload: Option<serde_json::Value>,
}
