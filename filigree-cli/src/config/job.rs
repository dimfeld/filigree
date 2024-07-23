use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use cargo_toml::Manifest;
use convert_case::{Case, Casing};
use error_stack::Report;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Error;

pub fn add_deps(api_dir: &Path, manifest: &mut Manifest) -> Result<(), Report<Error>> {
    crate::add_deps::add_dep(api_dir, manifest, "effectum", "0.7.0", &[])?;
    crate::add_deps::add_dep(api_dir, manifest, "time", "0.3.34", &[])?;
    Ok(())
}

/// Configuration for the queue itself
#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct QueueConfig {
    #[serde(default = "default_queue_path")]
    path: PathBuf,
}

impl QueueConfig {
    pub fn template_context(&self) -> serde_json::Value {
        json!({
            "path": self.path
        })
    }
}

fn default_queue_path() -> PathBuf {
    PathBuf::from("queue.db")
}

/// Configuratio a background job
#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct JobConfig {
    /// The default priority for this job. Defaults to 1, and jobs with higher priority will be run first
    #[serde(default = "default_priority")]
    pub priority: i32,

    /// The default weight for this job. Defaults to 1, and this value counts against the
    /// concurrency limit for a worker.
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// How long to wait for this job to run before retrying or failing.
    pub timeout: Option<std::time::Duration>,

    /// Whether or not to automatically heartbeat this job, to extend the timeout
    /// while it is still running.
    #[serde(default)]
    pub autoheartbeat: bool,

    /// Schedules to automatically run this job, if any
    #[serde(default)]
    pub schedule: Vec<JobSchedule>,

    /// One of the workers defined in `job.worker`. The `default` worker will be used
    /// if not specified.
    #[serde(default = "default_worker")]
    pub worker: String,
}

impl JobConfig {
    pub fn template_context(&self, name: &str) -> serde_json::Value {
        json!({
            "name": name,
            "type_name": name.to_case(Case::Pascal),
            "module": name.to_case(Case::Snake),
            "autoheartbeat": self.autoheartbeat,
            "priority": self.priority,
            "weight": self.weight,
            "timeout": self.timeout,
            "schedules": self.schedule.iter().map(|s| s.template_context()).collect::<Vec<_>>(),
        })
    }
}

fn default_priority() -> i32 {
    1
}

fn default_weight() -> u32 {
    1
}

fn default_worker() -> String {
    "default".into()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobSchedule {
    /// The name of the job schedule
    pub name: String,

    /// A cron string that specifies how this job will run
    pub schedule: String,

    /// A payload to invoke the job with
    pub payload: Option<serde_json::Value>,

    /// Disable this job and remove it from the system
    #[serde(default)]
    pub disabled: bool,
}

impl JobSchedule {
    fn template_context(&self) -> serde_json::Value {
        // TODO validate cron schedule
        json!({
            "name": self.name,
            "enabled": !self.disabled,
            "schedule": self.schedule,
            "payload": serde_json::to_string_pretty(&self.payload).unwrap()
        })
    }
}

#[derive(Debug, serde_derive_default::Default, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Set the minimum concurrency for this worker. When the number of running
    /// jobs falls below this number, the worker will try to fetch more jobs, up
    /// to max_concurrency. Defaults to the same as max_concurrency
    pub min_concurrency: Option<usize>,

    /// The worker will have no more than this many jobs running at once.
    /// Defaults to 4
    pub max_concurrency: Option<usize>,
}

impl WorkerConfig {
    pub fn template_context(
        &self,
        name: &str,
        jobs: &BTreeMap<String, JobConfig>,
    ) -> Option<serde_json::Value> {
        let jobs_for_this_worker = jobs
            .iter()
            .filter(|(_, job)| job.worker == name)
            .map(|(k, _)| k)
            .sorted()
            .collect::<Vec<_>>();

        if jobs_for_this_worker.is_empty() {
            return None;
        }

        Some(json!({
            "name": name,
            "name_upper": name.to_case(Case::ScreamingSnake),
            "jobs": jobs_for_this_worker,
            "min_concurrency": self.min_concurrency(),
            "max_concurrency": self.max_concurrency(),
        }))
    }

    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency.unwrap_or(4)
    }

    pub fn min_concurrency(&self) -> usize {
        self.min_concurrency.unwrap_or(self.max_concurrency())
    }
}

pub fn workers_context(
    workers: &BTreeMap<String, WorkerConfig>,
    jobs: &BTreeMap<String, JobConfig>,
) -> Vec<serde_json::Value> {
    workers
        .into_iter()
        .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
        .filter_map(|(k, v)| v.template_context(k, jobs))
        .collect()
}
