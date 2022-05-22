use crate::core::Xid;
use async_trait::async_trait;
use bytes::Bytes;

use crate::core::queue::QueueError;

/// Job lifecycle handler and metadata provider. For an example implementation look at `aide_de_camp_sqlite` crate.
#[async_trait]
pub trait JobHandle: Send + Sync {
    // Get job id
    fn id(&self) -> Xid;
    // Get Job type
    fn job_type(&self) -> &str;
    // Get job payload.
    fn payload(&self) -> Bytes;
    // How many times this job has been retried already.
    fn retries(&self) -> u32;
    // Mark the job as completed successfully.
    async fn complete(mut self) -> Result<(), QueueError>;
    // Mark the job as failed
    async fn fail(mut self) -> Result<(), QueueError>;
    // Move the job to dead queue.
    async fn dead_queue(mut self) -> Result<(), QueueError>;
}
