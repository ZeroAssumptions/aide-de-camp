use crate::core::Xid;
use async_trait::async_trait;
use std::convert::Infallible;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

/// A job-handler interface. Your Payload should implement `bincode::{Decode, Encode}` if you're
/// planning to use it with the runner and Queue from this crate.
///
/// ## Example
/// ```rust
/// use aide_de_camp::prelude::{JobProcessor, Encode, Decode, Xid, CancellationToken};
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
/// ```
/// ## Services
/// If your job processor requires external services (i.e. database client, REST client, etc.), add
/// them directly as your struct fields.
#[async_trait]
pub trait JobProcessor: Send + Sync {
    /// What is the input to this handler. If you want to use `RunnerRouter`, then this must implement `bincode::Decode` and `bincode::Encode`.
    type Payload: Send;
    /// What error is returned
    type Error: Send;
    /// Run the job, passing payload to it. Your payload should implement `bincode::Decode`.
    /// You should listen for the `cancellation_token.cancelled()` event in order to handle shutdown requests gracefully.
    async fn handle(
        &self,
        jid: Xid,
        payload: Self::Payload,
        cancellation_token: CancellationToken,
    ) -> Result<(), Self::Error>;

    /// How many times job should be retried before being moved to dead queue
    fn max_retries(&self) -> u32 {
        0
    }

    /// How long to wait before forcefully terminating the job when the server receives a shutdown request.
    fn shutdown_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(1)
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

    #[error("Job failed to complete within the shutdown timeout of {0:#?}")]
    ShutdownTimeout(std::time::Duration),

    /// Error originated in inner-job implementation
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<Infallible> for JobError {
    fn from(_: Infallible) -> Self {
        unreachable!();
    }
}
