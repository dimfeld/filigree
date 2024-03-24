//! transcode_video background job
#![allow(unused_imports, unused_variables, dead_code)]

use effectum::{JobBuilder, JobRunner, Queue, RecurringJobSchedule, RunningJob};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::server::ServerState;

/// The payload data for the transcode_video background job
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscodeVideoJobPayload {
    // Fill in your payload data here
}

/// Run the transcode_video background job
async fn run(job: RunningJob, context: ServerState) -> Result<(), crate::Error> {
    // Fill in your job logic here
    Ok(())
}

/// Enqueue the transcode_video job to run immediately
pub async fn enqueue(
    state: &ServerState,
    payload: &TranscodeVideoJobPayload,
) -> Result<uuid::Uuid, effectum::Error> {
    create_job_builder()
        .json_payload(payload)?
        .add_to(&state.queue)
        .await
}

/// Enqueue the transcode_video job to run at a specific time
pub async fn enqueue_at(
    state: &ServerState,
    payload: &TranscodeVideoJobPayload,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<uuid::Uuid, effectum::Error> {
    // convert to time crate
    let timestamp = at.timestamp();
    let t = time::OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| effectum::Error::TimestampOutOfRange("at"))?;

    create_job_builder()
        .json_payload(payload)?
        .run_at(t)
        .add_to(&state.queue)
        .await
}

/// Register this job with the queue and initialize any recurring jobs.
pub async fn register(queue: &Queue) -> Result<JobRunner<ServerState>, effectum::Error> {
    // TODO register the job with the system
    let runner = JobRunner::builder("transcode_video", run)
        .autoheartbeat(false)
        .build();

    Ok(runner)
}

fn create_job_builder() -> JobBuilder {
    JobBuilder::new("transcode_video").priority(1).weight(1)
}
