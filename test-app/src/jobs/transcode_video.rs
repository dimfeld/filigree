//! transcode_video background job
#![allow(unused_imports, unused_variables, dead_code)]

use effectum::{JobBuilder, JobRunner, Queue, RecurringJobSchedule, RunningJob};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::JobError;
use crate::server::ServerState;

/// The payload data for the transcode_video background job
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscodeVideoJobPayload {
    // Fill in your payload data here
}

/// Run the transcode_video background job
async fn run(job: RunningJob, state: ServerState) -> Result<(), error_stack::Report<JobError>> {
    let payload: TranscodeVideoJobPayload = job.json_payload().change_context(JobError::Payload)?;

    Ok(())
}

/// Enqueue the transcode_video job to run immediately
pub async fn enqueue(
    state: &ServerState,
    name: impl ToString,
    payload: &TranscodeVideoJobPayload,
) -> Result<uuid::Uuid, effectum::Error> {
    create_job_builder()
        .name(name)
        .json_payload(payload)?
        .add_to(&state.queue)
        .await
}

/// Enqueue the transcode_video job to run at a specific time
pub async fn enqueue_at(
    state: &ServerState,
    name: impl ToString,
    at: chrono::DateTime<chrono::Utc>,
    payload: &TranscodeVideoJobPayload,
) -> Result<uuid::Uuid, effectum::Error> {
    // convert to time crate
    let timestamp = at.timestamp();
    let t = time::OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| effectum::Error::TimestampOutOfRange("at"))?;

    create_job_builder()
        .name(name)
        .json_payload(payload)?
        .run_at(t)
        .add_to(&state.queue)
        .await
}

/// Register this job with the queue and initialize any recurring jobs.
pub async fn register(
    queue: &Queue,
    init_recurring_jobs: bool,
) -> Result<JobRunner<ServerState>, effectum::Error> {
    let runner = JobRunner::builder("transcode_video", run)
        .autoheartbeat(false)
        .format_failures_with_debug(true)
        .build();

    Ok(runner)
}

fn create_job_builder() -> JobBuilder {
    JobBuilder::new("transcode_video").priority(1).weight(1)
}
