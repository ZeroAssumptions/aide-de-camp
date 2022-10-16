use aide_de_camp::core::Xid;
use anyhow::Context;
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

pub type ScheduleName = Cow<'static, str>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Schedule {
    pub name: ScheduleName,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub current_run: Option<Xid>,
}
#[async_trait::async_trait]
pub trait ScheduleStore {
    /// Get all schedules that should run. Calling this method should ensure that taken schedules won't be returned again.
    async fn take_schedules_to_run(&self, now: DateTime<Utc>) -> anyhow::Result<Vec<Schedule>>;
    /// List all schedules. This method is idempotent.
    async fn list_schedules(&self) -> anyhow::Result<Vec<Schedule>>;
    /// Add schedules to the store.
    async fn add_schedules(&self, schedules: &[Schedule]) -> anyhow::Result<()>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryScheduleStore {
    schedules: Arc<RwLock<Vec<Schedule>>>,
}

#[async_trait::async_trait]
impl ScheduleStore for InMemoryScheduleStore {
    async fn take_schedules_to_run(&self, now: DateTime<Utc>) -> anyhow::Result<Vec<Schedule>> {
        let mut inner = self
            .schedules
            .write()
            .map_err(|_| anyhow::anyhow!("Lock is poisoned"))?;

        // This should use drain, but it's not available on stable
        let mut to_run = Vec::with_capacity(5);
        let mut i = 0;
        while i < inner.len() {
            if inner[i].next_run < now {
                to_run.push(inner.remove(i));
            } else {
                i += 1;
            }
        }
        Ok(to_run)
    }

    async fn list_schedules(&self) -> anyhow::Result<Vec<Schedule>> {
        let schedules = self
            .schedules
            .read()
            .map_err(|_| anyhow::anyhow!("Lock is poisoned"))?;
        Ok(schedules.deref().clone())
    }

    async fn add_schedules(&self, schedules: &[Schedule]) -> anyhow::Result<()> {
        let mut inner = self
            .schedules
            .write()
            .map_err(|_| anyhow::anyhow!("Lock is poisoned"))?;
        inner.extend_from_slice(schedules);
        inner.sort_by_key(|s| s.next_run);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ScheduleStore;
    use chrono::Duration;

    #[tokio::test]
    async fn test_add_schedules() {
        let store = InMemoryScheduleStore::default();
        let schedule = Schedule {
            name: Cow::from("test"),
            next_run: Utc::now() - Duration::days(1),
            last_run: None,
            current_run: None,
        };

        let schedules = store.list_schedules().await.unwrap();
        assert!(schedules.is_empty());

        store.add_schedules(&[schedule.clone()]).await.unwrap();
        let schedules = store.list_schedules().await.unwrap();

        assert_eq!(schedule, schedules[0]);
    }

    #[tokio::test]
    async fn test_take_schedules_to_run() {
        let store = InMemoryScheduleStore::default();
        let schedule1 = Schedule {
            name: Cow::from("test1"),
            next_run: Utc::now() - Duration::days(1),
            last_run: None,
            current_run: None,
        };

        let schedule2 = Schedule {
            name: Cow::from("test2"),
            next_run: Utc::now() + Duration::days(1),
            last_run: None,
            current_run: None,
        };

        store
            .add_schedules(&[schedule1.clone(), schedule2])
            .await
            .unwrap();

        let schedules = store.take_schedules_to_run(Utc::now()).await.unwrap();
        assert_eq!(1, schedules.len());
        assert_eq!(schedule1, schedules[0]);
    }
}
