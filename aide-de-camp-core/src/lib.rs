extern crate core;

pub mod error;
pub mod job;
pub mod mixins;
pub mod queue;
pub mod runner;

pub use anyhow;
pub use async_trait;
pub use bytes;
pub use chrono;
pub use tokio;
pub use xid;
pub use xid::Id as Xid;

pub type DateTime = chrono::DateTime<chrono::Utc>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
