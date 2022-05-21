pub use xid::new as new_xid;
pub use xid::Id as Xid;

pub type DateTime = chrono::DateTime<chrono::Utc>;
pub use bincode;
pub use bytes::Bytes;
pub use chrono::{Duration, Utc};

pub mod job_handle;
pub mod job_processor;
pub mod queue;
