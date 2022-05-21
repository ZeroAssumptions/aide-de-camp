use crate::core::Xid;
use async_trait::async_trait;
use std::convert::Infallible;
use thiserror::Error;

/// A job-handler interface. Your Payload should implement `bincode::{Decode, Encode}` if you're
/// planning to use it with the runner and Queue from this crate.
///
/// ## Example
/// ```rust
/// use aide_de_camp::prelude::{JobHandler, Encode, Decode, Xid};
/// use async_trait::async_trait;
/// struct MyJob;
/// #[derive(Encode, Decode)]
/// struct MyJobPayload(u8, String);
///
/// #[async_trait::async_trait]
/// impl JobHandler for MyJob {
///     type Payload = MyJobPayload;
///     type Error = anyhow::Error;
///
///     fn name() -> &'static str {
///         "my_job"
///     }
///
///     async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error> {
///         // ..do work
///         Ok(())
///     }
/// }
/// ```
/// ## Services
/// If your job processor requires external services (i.e. database client, REST client, etc), add
/// them directly as your struct fields.
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// What is the input to this handler. If you want to use `RunnerRouter`, then this must implement `bincode::Decode` and `bincode::Encode`.
    type Payload: Send;
    /// What error is returned
    type Error: Send;
    /// Run the job, passing payload to it. Your payload should implement `bincode::Decode`.
    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error>;

    /// How many times job should be retried before being moved to dead queue
    fn max_retries(&self) -> u32 {
        0
    }

    /// Job type, used to differentiate between different jobs in the queue.
    fn name() -> &'static str
    where
        Self: Sized;
}

/// Error types returned by job processor that wraps your job processor.
#[derive(Error, Debug)]
pub enum JobError {
    /// Encountered an error when tried to deserialize Context.
    #[error("Failed to deserialize job context")]
    DecodeError {
        #[from]
        source: bincode::error::DecodeError,
    },

    /// Error originated in inner-job implementation
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<Infallible> for JobError {
    fn from(_: Infallible) -> Self {
        unreachable!();
    }
}
