use async_trait::async_trait;
use bincode::{Decode, Encode};
use chrono::Utc;
use thiserror::Error;

use crate::core::job_handle::JobHandle;
use crate::core::job_processor::JobProcessor;
use crate::core::{DateTime, Duration, Xid};

/// An interface to queue implementation. Responsible for pushing jobs into the queue and pulling
/// jobs out of the queue.
///
/// ### Priority
///
/// When is enqueued one can specify priority. Jobs with higher priority will get polled first even if submitted after lower priority jobs.
#[async_trait]
pub trait Queue: Send + Sync {
    type JobHandle: JobHandle;
    /// Schedule a job to run at the future time.
    async fn schedule_at<J>(
        &self,
        payload: J::Payload,
        scheduled_at: DateTime,
        priority: i8,
    ) -> Result<Xid, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Encode;
    /// Schedule a job to run next. Depending on queue backlog this may start running later than you expect.
    async fn schedule<J>(&self, payload: J::Payload, priority: i8) -> Result<Xid, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Encode,
    {
        self.schedule_at::<J>(payload, Utc::now(), priority).await
    }

    /// Schedule a job to run at the future time relative to now.
    async fn schedule_in<J>(
        &self,
        payload: J::Payload,
        scheduled_in: Duration,
        priority: i8,
    ) -> Result<Xid, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Encode,
    {
        let when = Utc::now() + scheduled_in;
        self.schedule_at::<J>(payload, when, priority).await
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

    /// Cancel job that has been scheduled. Right now this will only cancel if the job hasn't started yet.
    async fn cancel_job(&self, job_id: Xid) -> Result<(), QueueError>;

    /// The same as [`cancel_job`](struct.cancel_job.html), but returns payload of canceled job.
    /// If deserialization fails, then job won't be cancelled.
    async fn unschedule_job<J>(&self, job_id: Xid) -> Result<J::Payload, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Decode;
}

/// Errors related to queue operation.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum QueueError {
    /// Encountered an error when tried to serialize Context.
    #[error("Failed to serialize job context")]
    EncodeError {
        #[from]
        source: bincode::error::EncodeError,
    },
    /// Encountered an error when tried to deserialize Context.
    #[error("Failed to deserialize job context")]
    DecodeError {
        #[from]
        source: bincode::error::DecodeError,
    },
    #[error("Interval must be greater than zero: {0:?}")]
    InvalidInterval(Duration),
    #[error("Job by that ID does not exist: {0}")]
    JobNotFound(Xid),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
