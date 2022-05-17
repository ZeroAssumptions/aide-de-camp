pub mod job_handle;
pub mod queue;
pub mod types;

pub const SCHEMA_SQL: &str = include_str!("../sql/schema.sql");

#[cfg(test)]
mod test {
    use crate::queue::SqliteQueue;
    use crate::SCHEMA_SQL;
    use aide_de_camp_core::async_trait::async_trait;
    use aide_de_camp_core::chrono::Duration;
    use aide_de_camp_core::job::JobHandler;
    use aide_de_camp_core::queue::{JobHandle, Queue};
    use aide_de_camp_core::{tokio, Xid};
    use bincode::{Decode, Encode};
    use sqlx::types::chrono::Utc;
    use sqlx::SqlitePool;
    use std::convert::Infallible;

    #[allow(dead_code)]
    pub fn setup_logger() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .init();
    }

    async fn make_pool(uri: &str) -> SqlitePool {
        let pool = SqlitePool::connect(uri).await.unwrap();
        {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query(SCHEMA_SQL).execute(&mut tx).await.unwrap();
            tx.commit().await.unwrap();
        }
        pool
    }

    #[derive(Encode, Decode)]
    struct TestPayload {
        arg1: i32,
        arg2: String,
    }

    impl Default for TestPayload {
        fn default() -> Self {
            Self {
                arg1: 1774,
                arg2: String::from("this is a test"),
            }
        }
    }

    struct TestJob1;

    #[async_trait]
    impl JobHandler for TestJob1 {
        type Payload = TestPayload;
        type Error = Infallible;

        async fn handle(&self, _jid: Xid, _payload: Self::Payload) -> Result<(), Self::Error> {
            Ok(())
        }

        fn name() -> &'static str
        where
            Self: Sized,
        {
            "test_job_1"
        }
    }

    #[tokio::test]
    async fn queue_smoke_test() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);

        // If there are no jobs, this should return Ok(None);
        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap();
            assert!(job.is_none());
        }
        // Schedule a job to run now
        let jid1 = queue
            .schedule::<TestJob1>(TestPayload::default())
            .await
            .unwrap();

        // Now poll_next should return this job to us
        let job1 = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
        assert_eq!(jid1, job1.id());
        // Second time poll should not return anything
        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap();
            assert!(job.is_none());
        }

        // Completed jobs should not show up in queue again
        job1.complete().await.unwrap();
        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap();
            assert!(job.is_none());
        }
    }

    #[tokio::test]
    async fn failed_jobs() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);

        // Schedule a job to run now
        let _jid1 = queue
            .schedule::<TestJob1>(TestPayload::default())
            .await
            .unwrap();

        // Now poll_next should return this job to us
        let job1 = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
        assert_eq!(job1.retries(), 1);
        // Fail the job
        job1.fail().await.unwrap();

        // We should be able to get the same job again, but it should have increased retry count

        let job1 = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
        assert_eq!(job1.retries(), 2);
    }

    #[tokio::test]
    async fn scheduling_future_jobs() {
        setup_logger();
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);

        // Schedule to run job tomorrow
        // Schedule a job to run now
        let tomorrow_jid = queue
            .schedule_in::<TestJob1>(TestPayload::default(), Duration::days(1))
            .await
            .unwrap();

        // Should not be polled yet
        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap();
            assert!(job.is_none());
        }

        let hour_ago = { Utc::now() - Duration::hours(1) };
        let hour_ago_jid = queue
            .schedule_at::<TestJob1>(TestPayload::default(), hour_ago)
            .await
            .unwrap();

        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
            assert_eq!(hour_ago_jid, job.id());
        }

        let tomorrow = Utc::now() + Duration::days(1) + Duration::minutes(1);
        {
            let job = queue
                .poll_next_with_instant(&[TestJob1::name()], tomorrow)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(tomorrow_jid, job.id());
        }

        // Everything should be in-progress, so None
        {
            let job = queue
                .poll_next_with_instant(&[TestJob1::name()], tomorrow)
                .await
                .unwrap();
            assert!(job.is_none());
        }
    }
}
