use super::job_router::RunnerRouter;
use crate::core::queue::{Queue, QueueError};
use anyhow::Context;
use futures::{future::join_all, stream::FuturesUnordered};
use rand::seq::SliceRandom;
use std::{
    future::{self, Future},
    sync::Arc,
};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub const JITTER_INTERVAL_MS: [i64; 10] = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34];

/// Options for configuring the runner
#[non_exhaustive]
pub struct RunnerOptions {
    /// How long to wait for all workers to shut down gracefully when system shutdown is requested.
    /// This timeout will only be hit if the job timeout is set higher than this or if some part of the system is stuck.
    ///
    /// This should be set higher than the job timeout in order to ensure all jobs are properly cleaned up.
    /// The default value is 15 seconds.
    pub worker_shutdown_timeout: std::time::Duration,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            worker_shutdown_timeout: std::time::Duration::from_secs(15),
        }
    }
}

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
    options: RunnerOptions,
}

impl<Q> JobRunner<Q>
where
    Q: Queue + 'static,
{
    /// Create a new JobRunner with desired concurrency from queue and router.
    pub fn new(
        queue: Q,
        processor: RunnerRouter,
        concurrency: usize,
        options: RunnerOptions,
    ) -> Self {
        Self {
            queue: Arc::new(queue),
            processor: Arc::new(processor),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            options,
        }
    }
}

impl<Q> JobRunner<Q>
where
    Q: Queue + 'static,
{
    /// Run the job runner with the desired polling interval. This method blocks forever.
    pub async fn run(&mut self, interval: chrono::Duration) -> Result<(), QueueError> {
        // Pass in a future that never completes to the shutdown handler so the server can run until program termination
        self.run_with_shutdown(interval, future::pending()).await
    }

    /// Run the job runner with the desired polling interval.
    /// This method blocks until the provided shutdown future completes.
    pub async fn run_with_shutdown<F: Future<Output = ()>>(
        &mut self,
        interval: chrono::Duration,
        shutdown: F,
    ) -> Result<(), QueueError> {
        let worker_handles = FuturesUnordered::new();
        let cancellation_token = CancellationToken::new();
        let mut shutdown = Box::pin(shutdown);
        loop {
            let semaphore = self.semaphore.clone();
            tokio::select! {
                permit = semaphore.acquire_owned() => {
                    let permit = permit.context("Semaphore closed while running")?;
                    let worker_handle = self.spawn_worker(permit, interval, cancellation_token.child_token());
                    worker_handles.push(worker_handle);
                }
                _ = &mut shutdown => {
                    handle_shutdown(self.options.worker_shutdown_timeout, worker_handles, cancellation_token).await;
                    return Ok(());
                }
            }
        }
    }

    fn spawn_worker(
        &self,
        permit: OwnedSemaphorePermit,
        interval: chrono::Duration,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        let queue = self.queue.clone();
        let processor = self.processor.clone();
        let cancellation_token = cancellation_token.child_token();

        tokio::spawn(async move {
            let _permit = permit;
            let interval = interval + get_random_jitter();
            processor.listen(queue, interval, cancellation_token).await;
        })
    }
}

async fn handle_shutdown(
    worker_shutdown_timeout: std::time::Duration,
    worker_handles: FuturesUnordered<JoinHandle<()>>,
    cancellation_token: CancellationToken,
) {
    tracing::debug!("Received shutdown request, attempting graceful shutdown");
    cancellation_token.cancel();
    match tokio::time::timeout(worker_shutdown_timeout, join_all(worker_handles)).await {
        Ok(worker_results) => {
            for result in worker_results {
                if let Err(e) = result {
                    tracing::error!("Worker panicked: {e:?}");
                }
            }
            tracing::debug!("All workers have shut down");
        }
        Err(_) => tracing::warn!(
            "One or more workers failed to shut down within the grace period of {:#?}",
            worker_shutdown_timeout
        ),
    }
}

fn get_random_jitter() -> chrono::Duration {
    JITTER_INTERVAL_MS
        .choose(&mut rand::thread_rng())
        .map(|ms| chrono::Duration::milliseconds(*ms))
        .unwrap_or_else(|| chrono::Duration::milliseconds(5)) // Always takes a happy path technically
}

#[cfg(test)]
mod test {
    use crate::core::*;
    use crate::prelude::*;
    use async_trait::async_trait;
    use futures::Future;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    #[derive(Default, Clone)]
    struct TestHandle {
        completed: Arc<AtomicBool>,
        failed: Arc<AtomicBool>,
        dead_queue: Arc<AtomicBool>,
        complete_delay: std::time::Duration,
    }

    #[async_trait]
    impl JobHandle for TestHandle {
        fn id(&self) -> Xid {
            xid::new()
        }

        fn job_type(&self) -> &str {
            "test"
        }

        fn payload(&self) -> Bytes {
            Bytes::new()
        }

        fn retries(&self) -> u32 {
            0
        }

        async fn complete(mut self) -> Result<(), QueueError> {
            tokio::time::sleep(self.complete_delay).await;
            self.completed.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn fail(mut self) -> Result<(), QueueError> {
            self.failed.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn dead_queue(mut self) -> Result<(), QueueError> {
            self.dead_queue.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestQueue {
        test_handle: Option<TestHandle>,
    }

    #[async_trait]
    impl Queue for TestQueue {
        type JobHandle = TestHandle;

        async fn schedule_at<J>(
            &self,
            _payload: J::Payload,
            _scheduled_at: DateTime,
            _priority: i8,
        ) -> Result<Xid, QueueError>
        where
            J: JobProcessor + 'static,
            J::Payload: Encode,
        {
            Ok(xid::new())
        }

        async fn poll_next_with_instant(
            &self,
            _job_types: &[&str],
            _now: DateTime,
        ) -> Result<Option<Self::JobHandle>, QueueError> {
            Ok(self.test_handle.clone())
        }

        async fn cancel_job(&self, _job_id: Xid) -> Result<(), QueueError> {
            Ok(())
        }

        async fn unschedule_job<J>(&self, job_id: Xid) -> Result<J::Payload, QueueError>
        where
            J: JobProcessor + 'static,
            J::Payload: Decode,
        {
            match &self.test_handle {
                Some(handle) => {
                    let payload = handle.payload();
                    let (decoded, _) =
                        bincode::decode_from_slice(&payload, bincode::config::standard())?;
                    Ok(decoded)
                }
                None => Err(QueueError::JobNotFound(job_id)),
            }
        }
    }

    struct TestJobProcessor {
        running: Arc<AtomicBool>,
        started_tx: tokio::sync::broadcast::Sender<()>,
        sleep_time: std::time::Duration,
        handle_shutdown: bool,
    }

    impl Default for TestJobProcessor {
        fn default() -> Self {
            let (started_tx, _) = tokio::sync::broadcast::channel(1);
            Self {
                started_tx,
                running: Default::default(),
                sleep_time: Default::default(),
                handle_shutdown: Default::default(),
            }
        }
    }

    #[async_trait]
    impl JobProcessor for TestJobProcessor {
        type Payload = ();
        type Error = anyhow::Error;

        async fn handle(
            &self,
            _jid: Xid,
            _payload: Self::Payload,
            cancellation_token: CancellationToken,
        ) -> Result<(), Self::Error> {
            self.running.store(true, Ordering::SeqCst);
            self.started_tx.send(()).unwrap();

            tokio::select! {
                _ = tokio::time::sleep(self.sleep_time) => {}
                _ = cancellation_token.cancelled(), if self.handle_shutdown => {}
            };

            self.running.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn name() -> &'static str {
            "test"
        }

        fn shutdown_timeout(&self) -> std::time::Duration {
            std::time::Duration::from_millis(10)
        }
    }

    async fn start_runner<F: Future<Output = ()> + Send + 'static>(
        mut runner: JobRunner<TestQueue>,
        shutdown: F,
    ) {
        let handle = tokio::spawn(async move {
            runner
                .run_with_shutdown(chrono::Duration::milliseconds(1), shutdown)
                .await
                .unwrap();
        });

        // Force the server to shut down after a few seconds to prevent the test from hanging
        // if there's a bug in the code that causes the shutdown to fail
        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap()
            .unwrap();
    }

    async fn shutdown_on_job_start(
        processor: TestJobProcessor,
        handle: TestHandle,
        options: RunnerOptions,
    ) {
        let mut router = RunnerRouter::default();
        let mut started_rx = processor.started_tx.subscribe();
        router.add_job_handler(processor);
        let queue = TestQueue {
            test_handle: Some(handle),
        };
        let runner = JobRunner::new(queue, router, 1, options);

        start_runner(runner, async move {
            // Initiate shutdown as soon as a job is started
            started_rx.recv().await.unwrap();
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handles_shutdown_with_no_jobs() {
        let runner = JobRunner::new(
            TestQueue::default(),
            RunnerRouter::default(),
            1,
            RunnerOptions::default(),
        );

        start_runner(runner, async move {
            // Give it a few milliseconds to ensure everything starts, then initiate shutdown
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shuts_down_forcefully_on_stuck_job() {
        let running = Arc::new(AtomicBool::new(false));

        // Job will run for 30 seconds without listening for cancellation
        let processor = TestJobProcessor {
            running: running.clone(),
            sleep_time: std::time::Duration::from_secs(30),
            handle_shutdown: false,
            ..Default::default()
        };

        let handle = TestHandle::default();
        let dead_queue = handle.dead_queue.clone();

        shutdown_on_job_start(processor, handle, RunnerOptions::default()).await;

        // Job should not have completed after shutdown completes
        assert!(running.load(Ordering::SeqCst));
        // Because it did not shut down gracefully, the job should've moved to the dead queue
        assert!(dead_queue.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shuts_job_down_gracefully_on_cancel() {
        let running = Arc::new(AtomicBool::new(false));

        // Job will run for 30 seconds and will listen for cancellation
        let processor = TestJobProcessor {
            running: running.clone(),
            sleep_time: std::time::Duration::from_secs(30),
            handle_shutdown: true,
            ..Default::default()
        };

        let handle = TestHandle::default();
        let completed = handle.completed.clone();

        shutdown_on_job_start(processor, handle, RunnerOptions::default()).await;

        // Job should've completed on cancel
        assert!(!running.load(Ordering::SeqCst));
        // Because it did shut down gracefully, the completion handler should've ran
        assert!(completed.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shuts_down_gracefully_on_queue_hang() {
        let running = Arc::new(AtomicBool::new(false));

        let processor = TestJobProcessor {
            running: running.clone(),
            handle_shutdown: true,
            ..Default::default()
        };

        let handle = TestHandle {
            // Simulate the queue hanging when the complete method is called
            complete_delay: std::time::Duration::from_secs(10),
            ..Default::default()
        };

        let completed = handle.completed.clone();

        shutdown_on_job_start(
            processor,
            handle,
            RunnerOptions {
                worker_shutdown_timeout: std::time::Duration::from_millis(10),
            },
        )
        .await;

        // Job should've completed immediately
        assert!(!running.load(Ordering::SeqCst));
        // Completion handler should not have ran because the queue should be stuck
        assert!(!completed.load(Ordering::SeqCst));
    }
}
