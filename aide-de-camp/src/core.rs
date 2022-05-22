//! Implementation agnostic traits for implementing queues and re-exports of 3rd party types/crates used in public interface.

/// A function to create new Xid.
pub use xid::new as new_xid;

/// Job ID implementation.
pub use xid::Id as Xid;

/// An alias for `chrono::DateTime<chrono::Utc>`
pub type DateTime = chrono::DateTime<chrono::Utc>;
pub use bincode;
pub use bytes::Bytes;
pub use chrono::{Duration, Utc};

pub mod job_handle;
pub mod job_processor;
pub mod queue;
