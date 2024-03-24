//! Background jobs

pub mod send_annoying_emails;
pub mod transcode_video;

use std::path::Path;

use effectum::{Queue, Worker};
use error_stack::Report;

use crate::{server::ServerState, Error};

pub struct QueueWorkers {
    pub default: Worker,
}

pub async fn create_queue(queue_location: &Path) -> Result<Queue, effectum::Error> {
    Queue::new(queue_location).await
}

pub async fn init(state: &ServerState) -> Result<QueueWorkers, effectum::Error> {
    // create the queue

    // register the jobs

    let send_annoying_emails_runner = send_annoying_emails::register(&queue).await?;

    let transcode_video_runner = transcode_video::register(&queue).await?;

    // create the workers

    let worker_default = Worker::builder(&queue, state.clone())
        .max_concurrency(4)
        .min_concurrency(4)
        .jobs([send_annoying_emails_runner, transcode_video_runner])
        .build()
        .await?;

    let workers = QueueWorkers {
        default: worker_default,
    };

    Ok((queue, workers))
}
