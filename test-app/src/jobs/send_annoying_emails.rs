//! send_annoying_emails background job
#![allow(unused_imports, unused_variables, dead_code)]

use effectum::{JobBuilder, JobRunner, Queue, RecurringJobSchedule, RunningJob};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::server::ServerState;

/// The payload data for the send_annoying_emails background job
#[derive(Debug, Serialize, Deserialize)]
pub struct SendAnnoyingEmailsJobPayload {
    // Fill in your payload data here
}

/// Run the send_annoying_emails background job
async fn run(job: RunningJob, context: ServerState) -> Result<(), crate::Error> {
    // Fill in your job logic here
    Ok(())
}

/// Enqueue the send_annoying_emails job to run immediately
pub async fn enqueue(
    state: &ServerState,
    payload: &SendAnnoyingEmailsJobPayload,
) -> Result<uuid::Uuid, effectum::Error> {
    create_job_builder()
        .json_payload(payload)?
        .add_to(&state.queue)
        .await
}

/// Enqueue the send_annoying_emails job to run at a specific time
pub async fn enqueue_at(
    state: &ServerState,
    payload: &SendAnnoyingEmailsJobPayload,
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
pub async fn register(
    queue: &Queue,
    init_recurring_jobs: bool,
) -> Result<JobRunner<ServerState>, effectum::Error> {
    // TODO register the job with the system
    let runner = JobRunner::builder("send_annoying_emails", run)
        .autoheartbeat(false)
        .build();

    if init_recurring_jobs {
        // Convert to the payload type just to make sure it's valid. This would be better
        // done by making the type directly so we get compile safety but that's difficult
        // to do from the template. Feel free to replace it with the equivalent.
        let payload: SendAnnoyingEmailsJobPayload =
            serde_json::from_value(json!(null)).map_err(effectum::Error::PayloadError)?;
        let daily_job = create_job_builder().json_payload(&payload)?.build();
        queue
            .upsert_recurring_job(
                "daily".to_string(),
                RecurringJobSchedule::Cron {
                    spec: "0 9 * * *".to_string(),
                },
                daily_job,
                false,
            )
            .await?;

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
