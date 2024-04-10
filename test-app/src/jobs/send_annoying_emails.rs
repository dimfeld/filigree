//! send_annoying_emails background job
#![allow(unused_imports, unused_variables, dead_code)]

use effectum::{JobBuilder, JobRunner, Queue, RecurringJobSchedule, RunningJob};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::JobError;
use crate::server::ServerState;

/// The payload data for the send_annoying_emails background job
#[derive(Debug, Serialize, Deserialize)]
pub struct SendAnnoyingEmailsJobPayload {
    // Fill in your payload data here
}

/// Run the send_annoying_emails background job
async fn run(job: RunningJob, state: ServerState) -> Result<(), error_stack::Report<JobError>> {
    let payload: SendAnnoyingEmailsJobPayload =
        job.json_payload().change_context(JobError::Payload)?;

    Ok(())
}

/// Enqueue the send_annoying_emails job to run immediately
pub async fn enqueue(
    state: &ServerState,
    name: impl ToString,
    payload: &SendAnnoyingEmailsJobPayload,
) -> Result<uuid::Uuid, effectum::Error> {
    create_job_builder()
        .name(name)
        .json_payload(payload)?
        .add_to(&state.queue)
        .await
}

/// Enqueue the send_annoying_emails job to run at a specific time
pub async fn enqueue_at(
    state: &ServerState,
    name: impl ToString,
    at: chrono::DateTime<chrono::Utc>,
    payload: &SendAnnoyingEmailsJobPayload,
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
    let runner = JobRunner::builder("send_annoying_emails", run)
        .autoheartbeat(false)
        .format_failures_with_debug(true)
        .build();

    if init_recurring_jobs {
        // daily is disabled
        match queue.delete_recurring_job("daily".to_string()).await {
            Ok(_) => {}
            // It's ok if the job doesn't exist. This just means it was already deleted
            // on a previous execution.
            Err(effectum::Error::NotFound) => {}
            Err(e) => return Err(e),
        };

        // monthly is disabled
        match queue.delete_recurring_job("monthly".to_string()).await {
            Ok(_) => {}
            // It's ok if the job doesn't exist. This just means it was already deleted
            // on a previous execution.
            Err(effectum::Error::NotFound) => {}
            Err(e) => return Err(e),
        };
    }

    Ok(runner)
}

fn create_job_builder() -> JobBuilder {
    JobBuilder::new("send_annoying_emails")
        .priority(1)
        .weight(1)
}
