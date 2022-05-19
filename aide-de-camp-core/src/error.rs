use chrono::Duration;
use std::convert::Infallible;
use thiserror::Error;


/// An error returned by WrappedJobHandler.
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


/// And error returned by queue implementation.
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


/// And error returned by RunnerRouter.
#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Runner is not configured to run this job type: {0}")]
    UnknownJobType(String),
    #[error(transparent)]
    QueueError(#[from] QueueError),
}
