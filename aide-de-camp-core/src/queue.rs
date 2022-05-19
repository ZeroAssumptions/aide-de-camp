use crate::error::{JobError, QueueError};
use crate::job::JobHandler;
use async_trait::async_trait;
use bincode::{Decode, Encode};
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use xid::Id as Xid;

/// An interface to interact with the queue. Depending on implementation this could be a durable queue or not.
#[async_trait]
pub trait Queue: Send + Sync {
    type JobHandle: JobHandle;
    /// Schedule a job to run at the future time.
    async fn schedule_at<J>(
        &self,
        payload: J::Payload,
        scheduled_at: DateTime<Utc>,
    ) -> Result<Xid, QueueError>
    where
        J: JobHandler + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>;

    /// Schedule a job to run next. Depending on the queue backlog this may start running later than you expect.
    async fn schedule<J>(&self, payload: J::Payload) -> Result<Xid, QueueError>
    where
        J: JobHandler + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
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
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        let when = Utc::now() + scheduled_in;
        self.schedule_at::<J>(payload, when).await
    }

    /// Pool queue, implementation should not wait for next job, if there nothing return `Ok(None)`.
    async fn poll_next_with_instant(
        &self,
        job_types: &[&str],
        time: DateTime<Utc>,
    ) -> Result<Option<Self::JobHandle>, QueueError>;

    /// Pool queue, implementation should not wait for next job, if there nothing return `Ok(None)`.
    async fn poll_next(&self, job_types: &[&str]) -> Result<Option<Self::JobHandle>, QueueError> {
        self.poll_next_with_instant(job_types, Utc::now()).await
    }

    /// Await the next job. Default implementation polls the queue with defined interval until there is something.
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

/// This trait is responsible for the entire job lifecycle.
#[async_trait]
pub trait JobHandle: Send + Sync {
    // Get job id.
    fn id(&self) -> Xid;
    // Get Job type.
    fn job_type(&self) -> &str;
    // Get job payload.
    fn payload(&self) -> Bytes;
    // How many times this job has been retried already.
    fn retries(&self) -> u32;
    // Mark the job as completed successfully.
    async fn complete(mut self) -> Result<(), QueueError>;
    // Mark the job as failed.
    async fn fail(mut self) -> Result<(), QueueError>;
    // Move the job to dead queue.
    async fn dead_queue(mut self) -> Result<(), QueueError>;
}
