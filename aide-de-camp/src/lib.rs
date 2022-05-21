#![doc = include_str!("../README.md")]

pub mod core;

/// Default implementation of job runner.
#[cfg(feature = "runner")]
pub mod runner {
    pub mod job_router;
    pub mod job_runner;
    pub mod wrapped_job;
}

pub mod prelude {
    pub use bincode::{Encode, Decode};
    pub use super::core::{Xid,Duration,job_processor::{JobHandler,JobError}, queue::{Queue, QueueError}, job_handle::JobHandle};
    #[cfg(feature = "runner")]
    pub use super::runner::{job_runner::JobRunner, job_router::RunnerRouter};
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
