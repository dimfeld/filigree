//! Background jobs

pub mod send_annoying_emails;
pub mod transcode_video;

use std::path::Path;

use effectum::{Queue, Worker};
use error_stack::ResultExt;
use futures::FutureExt;

use crate::{server::ServerState, Error};

#[derive(thiserror::Error, Debug)]
enum JobError {
    #[error("Failed to read payload")]
    Payload,
}

pub struct QueueWorkers {
    pub default: Worker,
}

impl QueueWorkers {
    pub async fn shutdown(self) {
        tokio::join!(self.default.unregister(None).map(|r| r.ok()),);
    }
}

pub async fn create_queue(queue_location: &Path) -> Result<Queue, effectum::Error> {
    Queue::new(queue_location).await
}

pub async fn init(
    state: &ServerState,
    init_recurring_jobs: bool,
) -> Result<QueueWorkers, error_stack::Report<Error>> {
    // register the jobs
    let send_annoying_emails_runner =
        send_annoying_emails::register(&state.queue, init_recurring_jobs)
            .await
            .change_context(Error::TaskQueue)?;
    let transcode_video_runner = transcode_video::register(&state.queue, init_recurring_jobs)
        .await
        .change_context(Error::TaskQueue)?;

    // create the workers
    let worker_default_min_concurrency =
        filigree::config::parse_option::<u16>(std::env::var("WORKER_DEFAULT_MIN_CONCURRENCY").ok())
            .change_context(Error::Config)?
            .unwrap_or(4);
    let worker_default_max_concurrency =
        filigree::config::parse_option::<u16>(std::env::var("WORKER_DEFAULT_MAX_CONCURRENCY").ok())
            .change_context(Error::Config)?
            .unwrap_or(4);

    let worker_default = Worker::builder(&state.queue, state.clone())
        .min_concurrency(worker_default_min_concurrency)
        .max_concurrency(worker_default_max_concurrency)
        .jobs([send_annoying_emails_runner, transcode_video_runner])
        .build()
        .await
        .change_context(Error::TaskQueue)?;

    let workers = QueueWorkers {
        default: worker_default,
    };

    Ok(workers)
}
