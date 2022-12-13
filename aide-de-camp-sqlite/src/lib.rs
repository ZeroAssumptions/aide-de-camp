#![doc = include_str!("../README.md")]

pub mod job_handle;
pub mod queue;
pub mod types;

pub use queue::SqliteQueue;
use sqlx::migrate::Migrator;
pub static MIGRATOR: Migrator = sqlx::migrate!();

#[cfg(test)]
mod test {
    use crate::queue::SqliteQueue;
    use crate::MIGRATOR;
    use aide_de_camp::core::bincode::{Decode, Encode};
    use aide_de_camp::core::job_handle::JobHandle;
    use aide_de_camp::core::job_processor::JobProcessor;
    use aide_de_camp::core::queue::Queue;
    use aide_de_camp::core::{CancellationToken, Duration, Xid};
    use aide_de_camp::prelude::QueueError;
    use async_trait::async_trait;
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
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    #[derive(Encode, Decode, PartialEq, Clone, Debug)]
    struct TestPayload1 {
        arg1: i32,
        arg2: String,
    }

    impl Default for TestPayload1 {
        fn default() -> Self {
            Self {
                arg1: 1774,
                arg2: String::from("this is a test"),
            }
        }
    }

    struct TestJob1;

    #[async_trait]
    impl JobProcessor for TestJob1 {
        type Payload = TestPayload1;
        type Error = Infallible;

        async fn handle(
            &self,
            _jid: Xid,
            _payload: Self::Payload,
            _cancellation_token: CancellationToken,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn name() -> &'static str
        where
            Self: Sized,
        {
            "test_job_1"
        }
    }

    #[derive(Encode, Decode, PartialEq, Clone, Debug)]
    struct TestPayload2 {
        arg1: i32,
        arg2: u64,
        arg3: String,
    }

    impl Default for TestPayload2 {
        fn default() -> Self {
            Self {
                arg1: 1774,
                arg2: 42,
                arg3: String::from("this is a test"),
            }
        }
    }

    struct TestJob2;

    #[async_trait]
    impl JobProcessor for TestJob2 {
        type Payload = TestPayload2;
        type Error = Infallible;

        async fn handle(
            &self,
            _jid: Xid,
            _payload: Self::Payload,
            _cancellation_token: CancellationToken,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn name() -> &'static str
        where
            Self: Sized,
        {
            "test_job_2"
        }
    }

    // Job with payload2 but job_type is from TestJob1
    struct TestJob3;

    #[async_trait]
    impl JobProcessor for TestJob3 {
        type Payload = TestPayload2;
        type Error = Infallible;

        async fn handle(
            &self,
            _jid: Xid,
            _payload: Self::Payload,
            _cancellation_token: CancellationToken,
        ) -> Result<(), Self::Error> {
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
            .schedule::<TestJob1>(TestPayload1::default(), 0)
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
            .schedule::<TestJob1>(TestPayload1::default(), 0)
            .await
            .unwrap();

        // Now poll_next should return this job to us
        let job1 = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
        assert_eq!(job1.retries(), 0);
        // Fail the job
        job1.fail().await.unwrap();

        // We should be able to get the same job again, but it should have increased retry count

        let job1 = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
        assert_eq!(job1.retries(), 1);
    }

    #[tokio::test]
    async fn scheduling_future_jobs() {
        setup_logger();
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);

        // schedule to run job tomorrow
        // schedule a job to run now
        let tomorrow_jid = queue
            .schedule_in::<TestJob1>(TestPayload1::default(), Duration::days(1), 0)
            .await
            .unwrap();

        // Should not be polled yet
        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap();
            assert!(job.is_none());
        }

        let hour_ago = { Utc::now() - Duration::hours(1) };
        let hour_ago_jid = queue
            .schedule_at::<TestJob1>(TestPayload1::default(), hour_ago, 0)
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

    #[tokio::test]
    async fn cancel_job_not_started() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);
        let jid = queue
            .schedule::<TestJob1>(TestPayload1::default(), 0)
            .await
            .unwrap();
        queue.cancel_job(jid).await.unwrap();

        // Should return None
        {
            let job = queue.poll_next(&[TestJob1::name()]).await.unwrap();
            assert!(job.is_none());
        }

        // Should fail
        let ret = queue.cancel_job(jid).await;
        assert!(matches!(ret, Err(QueueError::JobNotFound(_))));
    }

    #[tokio::test]
    async fn cancel_job_return_payload() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);
        let payload = TestPayload1::default();
        let jid = queue
            .schedule::<TestJob1>(payload.clone(), 0)
            .await
            .unwrap();

        let deleted_payload = queue.unschedule_job::<TestJob1>(jid).await.unwrap();
        assert_eq!(payload, deleted_payload);

        let ret = queue.unschedule_job::<TestJob1>(jid).await;
        assert!(matches!(ret, Err(QueueError::JobNotFound(_))));
    }

    #[tokio::test]
    async fn cancel_wrong_type() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);
        let jid = queue
            .schedule::<TestJob1>(TestPayload1::default(), 0)
            .await
            .unwrap();

        let result = queue.unschedule_job::<TestJob2>(jid).await;
        assert!(matches!(result, Err(QueueError::JobNotFound(_))));

        let result = queue.unschedule_job::<TestJob3>(jid).await;
        dbg!(&result);
        assert!(matches!(result, Err(QueueError::DecodeError { .. })));
    }

    #[tokio::test]
    async fn cancel_job_started() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);
        let payload = TestPayload1::default();
        let jid = queue
            .schedule::<TestJob1>(payload.clone(), 0)
            .await
            .unwrap();

        let _job = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();

        let ret = queue.cancel_job(jid).await;
        assert!(matches!(ret, Err(QueueError::JobNotFound(_))));

        let ret = queue.unschedule_job::<TestJob1>(jid).await;
        assert!(matches!(ret, Err(QueueError::JobNotFound(_))));
    }
    #[tokio::test]
    async fn priority_polling() {
        let pool = make_pool(":memory:").await;
        let queue = SqliteQueue::with_pool(pool);
        let hour_ago = { Utc::now() - Duration::hours(1) };
        let _hour_ago_jid = queue
            .schedule_at::<TestJob1>(TestPayload1::default(), hour_ago, 0)
            .await
            .unwrap();

        let higher_priority_jid = queue
            .schedule_at::<TestJob1>(TestPayload1::default(), hour_ago, 3)
            .await
            .unwrap();

        let job = queue.poll_next(&[TestJob1::name()]).await.unwrap().unwrap();
        assert_eq!(higher_priority_jid, job.id());
    }
}
