use super::wrapped_job::{BoxedJobHandler, WrappedJobHandler};
use crate::core::job_handle::JobHandle;
use crate::core::job_processor::{JobError, JobProcessor};
use crate::core::queue::{Queue, QueueError};
use bincode::{self, Decode, Encode};
use chrono::Duration;
use std::collections::HashMap;
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

/// A job processor router. Matches job type to job processor implementation.
/// This type requires that your jobs implement `Encode` + `Decode` from bincode trait. Those traits are re-exported in prelude.
///
/// ## Example
/// ```rust
/// use aide_de_camp::prelude::{JobProcessor, RunnerRouter, Encode, Decode, Xid, CancellationToken};
/// use async_trait::async_trait;
/// struct MyJob;
///
/// impl MyJob {
///     async fn do_work(&self) -> anyhow::Result<()> {
///         // ..do some work
///         Ok(())
///     }
/// }
///
/// #[derive(Encode, Decode)]
/// struct MyJobPayload(u8, String);
///
/// #[async_trait::async_trait]
/// impl JobProcessor for MyJob {
///     type Payload = MyJobPayload;
///     type Error = anyhow::Error;
///
///     fn name() -> &'static str {
///         "my_job"
///     }
///
///     async fn handle(&self, jid: Xid, payload: Self::Payload, cancellation_token: CancellationToken) -> Result<(), Self::Error> {
///         tokio::select! {
///             result = self.do_work() => { result }
///             _ = cancellation_token.cancelled() => { Ok(()) }
///         }
///     }
/// }
///
/// let router = {
///     let mut r = RunnerRouter::default();
///     r.add_job_handler(MyJob);
///     r
/// };
///
///```
#[derive(Default)]
pub struct RunnerRouter {
    jobs: HashMap<&'static str, BoxedJobHandler>,
}

impl RunnerRouter {
    /// Register a job handler with the router. If job by that name already present, it will get replaced.
    pub fn add_job_handler<J>(&mut self, job: J)
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        let name = J::name();
        let boxed = WrappedJobHandler::new(job).boxed();
        self.jobs.entry(name).or_insert(boxed);
    }

    pub fn types(&self) -> Vec<&'static str> {
        self.jobs.keys().copied().collect()
    }

    /// Process job handle. This function reposible for job lifecycle. If you're implementing your
    /// own job runner, then this is what you should use to process job that is already pulled
    /// from the queue. In all other cases, you shouldn't use this function directly.
    #[instrument(skip_all, err, fields(job_type = %job_handle.job_type(), jid = %job_handle.id().to_string(), retries = job_handle.retries()))]
    pub async fn process<H: JobHandle>(
        &self,
        job_handle: H,
        cancellation_token: CancellationToken,
    ) -> Result<(), RunnerError> {
        if let Some(r) = self.jobs.get(job_handle.job_type()) {
            let job_shutdown_timeout = r.shutdown_timeout();

            let job_result = tokio::select! {
                job_result = r.handle(job_handle.id(), job_handle.payload(), cancellation_token.child_token()) => {
                    job_result
                }
                cancellation_result = cancellation_handler(job_shutdown_timeout, cancellation_token.child_token()) => {
                    cancellation_result
                }
            };
            handle_job_result(
                job_result,
                job_handle,
                r.max_retries(),
                cancellation_token.child_token(),
            )
            .await
        } else {
            Err(RunnerError::UnknownJobType(
                job_handle.job_type().to_string(),
            ))
        }
    }

    /// In a loop, poll the queue with interval (passes interval to `Queue::next`) and process
    /// incoming jobs. Function process jobs one-by-one without job-level concurrency. If you need
    /// concurrency, look at the `JobRunner` instead.
    pub async fn listen<Q, QR>(
        &self,
        queue: Q,
        poll_interval: Duration,
        cancellation_token: CancellationToken,
    ) where
        Q: AsRef<QR>,
        QR: Queue,
    {
        let job_types = self.types();
        loop {
            tokio::select! {
                next = queue.as_ref().next(&job_types, poll_interval) => {
                    let cancellation_token = cancellation_token.child_token();
                    self.handle_next_job::<QR>(next, cancellation_token).await;
                }
                _ = cancellation_token.cancelled() => {
                    // If cancellation is requested while a job is processing, this block will execute on the next iteration.
                    tracing::debug!("Shutdown request received, stopping listener");
                    return
                }
            }
        }
    }

    async fn handle_next_job<QR>(
        &self,
        next: Result<QR::JobHandle, QueueError>,
        cancellation_token: CancellationToken,
    ) where
        QR: Queue,
    {
        match next {
            Ok(handle) => {
                match self.process(handle, cancellation_token.child_token()).await {
                    Ok(_) => {}
                    Err(RunnerError::QueueError(e)) => handle_queue_error(e).await,
                    Err(RunnerError::UnknownJobType(name)) => {
                        tracing::error!("Unknown job type: {}", name)
                    }
                };
            }
            Err(e) => {
                handle_queue_error(e).await;
            }
        }
    }
}

/// Errors returned by the router.
#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Runner is not configured to run this job type: {0}")]
    UnknownJobType(String),
    #[error(transparent)]
    QueueError(#[from] QueueError),
}

async fn cancellation_handler(
    job_shutdown_timeout: std::time::Duration,
    cancellation_token: CancellationToken,
) -> Result<(), JobError> {
    cancellation_token.cancelled().await;
    // Wait for the duration of the shutdown timeout specified by the job configuration.
    // The job should complete within the timeout period, preventing this method from completing.
    tokio::time::sleep(job_shutdown_timeout).await;
    Err(JobError::ShutdownTimeout(job_shutdown_timeout))
}

async fn handle_job_result<H: JobHandle>(
    job_result: Result<(), JobError>,
    job_handle: H,
    max_retries: u32,
    cancellation_token: CancellationToken,
) -> Result<(), RunnerError> {
    if cancellation_token.is_cancelled() {
        tracing::info!("Cancellation was requested during job processing");
    }
    match job_result.map_err(JobError::from) {
        Ok(_) => {
            job_handle.complete().await?;
            Ok(())
        }
        Err(e) => {
            tracing::error!("Error during job processing: {}", e);
            if job_handle.retries() >= max_retries {
                tracing::warn!("Moving job {} to dead queue", job_handle.id().to_string());
                job_handle.dead_queue().await?;
                Ok(())
            } else {
                job_handle.fail().await?;
                Ok(())
            }
        }
    }
}

async fn handle_queue_error(error: QueueError) {
    tracing::error!("Encountered QueueError: {}", error);
    tracing::warn!("Suspending worker for 5 seconds");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::Xid;
    use bincode::config::standard;
    use std::convert::Infallible;

    #[tokio::test]
    async fn it_is_object_safe_and_wrappable() {
        struct Example;

        #[async_trait::async_trait]
        impl JobProcessor for Example {
            type Payload = Vec<i32>;
            type Error = Infallible;

            async fn handle(
                &self,
                _jid: Xid,
                _payload: Self::Payload,
                _cancellation_token: CancellationToken,
            ) -> Result<(), Infallible> {
                dbg!("we did it patrick");
                Ok(())
            }
            fn name() -> &'static str {
                "example"
            }
        }

        let payload = vec![1, 2, 3];

        let job: Box<dyn JobProcessor<Payload = _, Error = _>> = Box::new(Example);

        job.handle(xid::new(), payload.clone(), CancellationToken::new())
            .await
            .unwrap();
        let wrapped: Box<dyn JobProcessor<Payload = _, Error = JobError>> =
            Box::new(WrappedJobHandler::new(Example));

        let payload = bincode::encode_to_vec(&payload, standard()).unwrap();

        wrapped
            .handle(xid::new(), payload.into(), CancellationToken::new())
            .await
            .unwrap();
    }
}
