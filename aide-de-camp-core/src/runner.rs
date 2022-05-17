use crate::error::{JobError, QueueError, RunnerError};
use crate::job::{BoxedJobHandler, JobHandler, WrappedJobHandler};
use crate::queue::{JobHandle, Queue};
use bincode::{Decode, Encode};
use chrono::Duration;
use std::collections::HashMap;

/// A job runner also known as worker.
#[derive(Default)]
pub struct RunnerRouter {
    jobs: HashMap<&'static str, BoxedJobHandler>,
}

impl RunnerRouter {
    /// Register a job handler with the router. If job by that name already present, it will get replaced.
    pub fn add_job_handler<J>(&mut self, job: J)
    where
        J: JobHandler + 'static,
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

    pub async fn process<H: JobHandle>(&self, job_handle: H) -> Result<(), RunnerError> {
        if let Some(r) = self.jobs.get(job_handle.job_type()) {
            match r
                .handle(job_handle.id(), job_handle.payload())
                .await
                .map_err(JobError::from)
            {
                Ok(_) => {
                    job_handle.complete().await?;
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Error during job processing: {}", e);
                    if job_handle.retries() >= r.max_retries() {
                        tracing::warn!("Moving job {} to dead queue", job_handle.id().to_string());
                        job_handle.dead_queue().await?;
                        Ok(())
                    } else {
                        job_handle.fail().await?;
                        Ok(())
                    }
                }
            }
        } else {
            Err(RunnerError::UnknownJobType(
                job_handle.job_type().to_string(),
            ))
        }
    }

    pub async fn listen<Q, QR>(&self, queue: Q, poll_interval: Duration)
    where
        Q: AsRef<QR>,
        QR: Queue,
    {
        let job_types = self.types();
        loop {
            match queue.as_ref().next(&job_types, poll_interval).await {
                Ok(handle) => match self.process(handle).await {
                    Ok(_) => {}
                    Err(RunnerError::QueueError(e)) => handle_queue_error(e).await,
                    Err(RunnerError::UnknownJobType(name)) => {
                        tracing::error!("Unknown job type: {}", name)
                    }
                },
                Err(e) => {
                    handle_queue_error(e).await;
                }
            }
        }
    }
}

async fn handle_queue_error(error: QueueError) {
    tracing::error!("Encountered QueueError: {}", error);
    tracing::warn!("Suspending worker for 5 seconds");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}
