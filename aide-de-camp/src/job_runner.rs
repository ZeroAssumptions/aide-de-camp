use aide_de_camp_core::anyhow::Context;
use aide_de_camp_core::error::QueueError;
use aide_de_camp_core::queue::Queue;
use aide_de_camp_core::runner::RunnerRouter;
use aide_de_camp_core::tokio::sync::Semaphore;
use aide_de_camp_core::{chrono, tokio};
use rand::seq::SliceRandom;
use std::sync::Arc;

const JITTER_INTERVAL_MS: [i64; 10] = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34];

pub struct JobRunner<Q>
where
    Q: Queue,
{
    queue: Arc<Q>,
    processor: Arc<RunnerRouter>,
    semaphore: Arc<Semaphore>,
}

impl<Q> JobRunner<Q>
where
    Q: Queue + 'static,
{
    pub fn new(queue: Q, processor: RunnerRouter, concurrency: usize) -> Self {
        Self {
            queue: Arc::new(queue),
            processor: Arc::new(processor),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }
}

impl<Q> JobRunner<Q>
where
    Q: Queue + 'static,
{
    pub async fn run(&mut self, interval: chrono::Duration) -> Result<(), QueueError> {
        loop {
            let semaphore = self.semaphore.clone();
            let permit = semaphore
                .acquire_owned()
                .await
                .context("Semaphore closed while running")?;
            let queue = self.queue.clone();
            let processor = self.processor.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let queue = queue;
                let processor = processor;
                let interval = interval + get_random_jitter();
                processor.listen(queue, interval).await;
            });
        }
    }
}

fn get_random_jitter() -> chrono::Duration {
    JITTER_INTERVAL_MS
        .choose(&mut rand::thread_rng())
        .map(|ms| chrono::Duration::milliseconds(*ms))
        .unwrap_or_else(|| chrono::Duration::milliseconds(5)) // Always takes a happy path technically
}
