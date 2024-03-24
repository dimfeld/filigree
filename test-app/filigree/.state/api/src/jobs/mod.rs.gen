//! Background jobs

pub mod send_annoying_emails;
pub mod transcode_video;

use std::path::Path;

use effectum::{Queue, Worker};
use futures::FutureExt;

use crate::server::ServerState;

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
) -> Result<QueueWorkers, effectum::Error> {
    // create the queue

    // register the jobs

    let send_annoying_emails_runner =
        send_annoying_emails::register(&state.queue, init_recurring_jobs).await?;

    let transcode_video_runner =
        transcode_video::register(&state.queue, init_recurring_jobs).await?;

    // create the workers

    let worker_default = Worker::builder(&state.queue, state.clone())
        .max_concurrency(4)
        .min_concurrency(4)
        .jobs([send_annoying_emails_runner, transcode_video_runner])
        .build()
        .await?;

    let workers = QueueWorkers {
        default: worker_default,
    };

    Ok(workers)
}
