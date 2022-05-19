use crate::error::JobError;
use crate::Xid;
use async_trait::async_trait;
use bincode::config::Configuration;
use bincode::{Decode, Encode};
use bytes::Bytes;
use std::error::Error;

pub type BoxedJobHandler = Box<dyn JobHandler<Payload = Bytes, Error = JobError>>;

/// A job-handler interface.
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// What is the input to this handler. If you want to use `RunnerRouter`, then this must implement `bincode::Decode` and `bincode::Encode`.
    type Payload: Send;
    /// What error is returned
    type Error: Error;
    /// Run the job, passing payload to it. Your payload should implement `bincode::Decode`.
    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error>;

    /// How many times job should be retried before being moved to dead queue
    fn max_retries(&self) -> u32 {
        0
    }
    /// Job type, used to differentiate between different jobs in the queue. Default is 0.
    fn name() -> &'static str
    where
        Self: Sized;
}

/// Object-safe implementation of a job that can be used in a router. Router handles wrapping, so this type shouldn't be used directly.
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
    /// Wrap a job that has bincode friendly payload and error type that can be converted to JobError.
    pub fn new(job: J) -> Self {
        let config = bincode::config::standard();
        Self { job, config }
    }

    /// Turn this into Box<dyn JobHandler<Payload = Bytes, Error = JobError>>
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

    #[tracing::instrument(skip_all, fields(jid = %jid.to_string()))]
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
#[cfg(test)]
mod test {
    use crate::error::JobError;
    use crate::job::{JobHandler, WrappedJobHandler};
    use crate::Xid;
    use bincode::config::standard;
    use std::convert::Infallible;

    #[tokio::test]
    async fn it_is_object_safe_and_wrappable() {
        struct Example;

        #[async_trait::async_trait]
        impl JobHandler for Example {
            type Payload = Vec<i32>;
            type Error = Infallible;

            async fn handle(&self, _jid: Xid, _payload: Self::Payload) -> Result<(), Infallible> {
                dbg!("we did it patrick");
                Ok(())
            }
            fn name() -> &'static str {
                "example"
            }
        }

        let payload = vec![1, 2, 3];

        let job: Box<dyn JobHandler<Payload = _, Error = _>> = Box::new(Example);

        job.handle(xid::new(), payload.clone()).await.unwrap();
        let wrapped: Box<dyn JobHandler<Payload = _, Error = JobError>> =
            Box::new(WrappedJobHandler::new(Example));

        let payload = bincode::encode_to_vec(&payload, standard()).unwrap();

        wrapped.handle(xid::new(), payload.into()).await.unwrap();
    }
}
