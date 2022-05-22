#![doc = include_str!("../README.md")]

pub mod core;

/// Default implementation of job runner.
#[cfg(feature = "runner")]
pub mod runner {
    pub mod job_router;
    pub mod job_runner;
    pub mod wrapped_job;
}

/// Re-exports to simplify importing this crate types.
pub mod prelude {
    pub use super::core::{
        job_handle::JobHandle,
        job_processor::{JobError, JobProcessor},
        queue::{Queue, QueueError},
        Duration, Xid,
    };
    #[cfg(feature = "runner")]
    pub use super::runner::{job_router::RunnerRouter, job_runner::JobRunner};
    pub use bincode::{Decode, Encode};
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
