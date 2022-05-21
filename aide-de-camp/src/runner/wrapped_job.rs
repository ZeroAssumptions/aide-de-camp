use crate::core::job_processor::{JobError, JobHandler};
use crate::core::Xid;
use async_trait::async_trait;
use bincode::{config::Configuration, Decode, Encode};
use bytes::Bytes;
use tracing::instrument;

/// Shorthand for boxed trait object for a WrappedJob.
pub type BoxedJobHandler = Box<dyn JobHandler<Payload = Bytes, Error = JobError>>;

/// Object-safe implementation of a job that can be used in runner. Generally speaking, you don't
/// need to directly use this type, JobRouter takes care of everything related to it.
pub struct WrappedJobHandler<T: JobHandler> {
    job: T,
    config: Configuration,
}

impl<J> WrappedJobHandler<J>
where
    J: JobHandler + 'static,
    J::Payload: Decode + Encode,
    J::Error: Into<JobError>,
{
    pub fn new(job: J) -> Self {
        let config = bincode::config::standard();
        Self { job, config }
    }

    pub fn boxed(self) -> BoxedJobHandler {
        Box::new(self) as BoxedJobHandler
    }
}

#[async_trait]
impl<J> JobHandler for WrappedJobHandler<J>
where
    J: JobHandler + 'static,
    J::Payload: Decode + Encode,
    J::Error: Into<JobError>,
{
    type Payload = Bytes;
    type Error = JobError;

    #[instrument(skip_all, err, fields(jid = %jid.to_string(), job_type = %Self::name()))]
    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error> {
        let (payload, _) = bincode::decode_from_slice(payload.as_ref(), self.config)?;
        self.job.handle(jid, payload).await.map_err(Into::into)
    }

    fn name() -> &'static str {
        J::name()
    }
}

impl<J> From<J> for WrappedJobHandler<J>
where
    J: JobHandler + 'static,
    J::Payload: Decode + Encode,
    J::Error: Into<JobError>,
{
    fn from(job: J) -> Self {
        Self::new(job)
    }
}
