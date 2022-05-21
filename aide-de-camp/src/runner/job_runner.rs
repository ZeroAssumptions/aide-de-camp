use super::job_router::RunnerRouter;
use crate::core::queue::{Queue, QueueError};
use anyhow::Context;
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub const JITTER_INTERVAL_MS: [i64; 10] = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34];


/// A bridge between job processors and the queue.
///
/// ## Implementation
///
/// This runner implemented very naively:
///
/// - First, it creates a semaphore with permits count equal to desited concurrency
/// - Then, in a loop, for every avaiable permit:
///     - Poll queue with given interval + random jitter
///     - Process incoming job
///     - Give back the permit
///
/// Future Implementation might work differently, but public interface should stay the same.
///
/// ## Examples
/// See `aide-de-camp-sqlite` for examples.
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
    /// Create a new JobRunner with desired concurrency from queue and router.
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
    /// This function runs forever unless built-in Semaphore gets closed. Which never happens.
    /// Later there might be a way to stop it.
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
