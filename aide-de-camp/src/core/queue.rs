use async_trait::async_trait;
use bincode::Encode;
use chrono::Utc;
use thiserror::Error;

use crate::core::job_handle::JobHandle;
use crate::core::job_processor::JobHandler;
use crate::core::{DateTime, Duration, Xid};

/// An interface to queue implementation. Reponsible for pushing jobs into the queue and pulling
/// jobs out of the queue.
#[async_trait]
pub trait Queue: Send + Sync {
    type JobHandle: JobHandle;
    /// Schedule a job to run at the future time.
    async fn schedule_at<J>(
        &self,
        payload: J::Payload,
        scheduled_at: DateTime,
    ) -> Result<Xid, QueueError>
    where
        J: JobHandler + 'static,
        J::Payload: Encode;
    /// Schedule a job to run next. Depending on queue backlog this may start running later than you expect.
    async fn schedule<J>(&self, payload: J::Payload) -> Result<Xid, QueueError>
    where
        J: JobHandler + 'static,
        J::Payload: Encode,
    {
        self.schedule_at::<J>(payload, Utc::now()).await
    }

    /// Schedule a job to run at the future time relative to now.
    async fn schedule_in<J>(
        &self,
        payload: J::Payload,
        scheduled_in: Duration,
    ) -> Result<Xid, QueueError>
    where
        J: JobHandler + 'static,
        J::Payload: Encode,
    {
        let when = Utc::now() + scheduled_in;
        self.schedule_at::<J>(payload, when).await
    }

    /// Pool queue, implementation should not wait for next job, if there nothing return `Ok(None)`.
    async fn poll_next_with_instant(
        &self,
        job_types: &[&str],
        time: DateTime,
    ) -> Result<Option<Self::JobHandle>, QueueError>;

    /// Pool queue, implementation should not wait for next job, if there nothing return `Ok(None)`.
    async fn poll_next(&self, job_types: &[&str]) -> Result<Option<Self::JobHandle>, QueueError> {
        self.poll_next_with_instant(job_types, Utc::now()).await
    }

    /// Await next job. Default implementation polls the queue with defined interval until there is something.
    async fn next(
        &self,
        job_types: &[&str],
        interval: Duration,
    ) -> Result<Self::JobHandle, QueueError> {
        let duration = interval
            .to_std()
            .map_err(|_| QueueError::InvalidInterval(interval))?;
        let mut interval = tokio::time::interval(duration);
        loop {
            interval.tick().await;
            let job = self.poll_next(job_types).await?;
            if let Some(job) = job {
                break Ok(job);
            }
        }
    }
}

/// Errors relateded to queue operation.
#[derive(Error, Debug)]
pub enum QueueError {
    /// Encountered an error when tried to serialize Context.
    #[error("Failed to serialize job context")]
    EncodeError {
        #[from]
        source: bincode::error::EncodeError,
    },

    #[error("Interval must be more than zero: {0:?}")]
    InvalidInterval(Duration),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
