use aide_de_camp::aide_de_camp_sqlite::queue::SqliteQueue;
use aide_de_camp::aide_de_camp_sqlite::SCHEMA_SQL;
use aide_de_camp::core::anyhow::anyhow;
use aide_de_camp::core::async_trait::async_trait;
use aide_de_camp::core::chrono::{Duration, Utc};
use aide_de_camp::core::error::JobError;
use aide_de_camp::core::job::JobHandler;
use aide_de_camp::core::queue::Queue;
use aide_de_camp::core::runner::RunnerRouter;
use aide_de_camp::core::Xid;
use aide_de_camp::job_runner::JobRunner;
use bincode::{Decode, Encode};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::StreamExt;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

struct JobResult {
    pub duration_millis: i64,
    pub jid: Xid,
}

#[derive(Decode, Encode)]
struct BenchJobPayload {
    pub started_at_millis: i64,
}

impl Default for BenchJobPayload {
    fn default() -> Self {
        Self {
            started_at_millis: Utc::now().timestamp_millis(),
        }
    }
}

struct BenchJob {
    tx: UnboundedSender<JobResult>,
    count: Arc<AtomicUsize>,
}

#[async_trait]
impl JobHandler for BenchJob {
    type Payload = BenchJobPayload;
    type Error = JobError;

    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error> {
        let _count = self.count.fetch_add(1, Ordering::SeqCst);
        let duration_millis = Utc::now().timestamp_millis() - payload.started_at_millis;
        self.tx
            .unbounded_send(JobResult {
                duration_millis,
                jid,
            })
            .map_err(|_| anyhow!("Failed to send results"))?;
        Ok(())
    }

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "bench_job"
    }
}

async fn make_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query(SCHEMA_SQL).execute(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
    }
    pool
}
async fn schedule_tasks(count: usize, interval: std::time::Duration, queue: Arc<SqliteQueue>) {
    let mut delay = tokio::time::interval(interval);
    for _ in 0..count {
        delay.tick().await;
        if let Err(e) = queue.schedule::<BenchJob>(BenchJobPayload::default()).await {
            eprintln!("Failed to schedule job: {}", e);
        }
    }
}

#[tokio::main]
async fn main() {
    let count = std::env::args()
        .nth(1)
        .map(|c| usize::from_str(&c).unwrap())
        .unwrap_or(10_000);
    let concurrency = std::env::args()
        .nth(2)
        .map(|c| usize::from_str(&c).unwrap())
        .unwrap_or(50);
    let interval_nanos = std::env::args()
        .nth(3)
        .map(|c| u64::from_str(&c).unwrap())
        .unwrap_or(700_000);

    let (tx, rx) = unbounded::<JobResult>();
    let pool = make_pool().await;
    let queue = SqliteQueue::with_pool(pool);

    let _task_maker = {
        let queue = queue.clone();
        let interval = std::time::Duration::from_nanos(interval_nanos);
        tokio::spawn(async move {
            schedule_tasks(count, interval, Arc::new(queue)).await;
        });
    };

    let processed = Arc::new(AtomicUsize::new(0));
    let job_handler = BenchJob {
        tx,
        count: processed.clone(),
    };

    let router = {
        let mut r = RunnerRouter::default();
        r.add_job_handler(job_handler);
        r
    };

    let started = Instant::now();
    let _runner = {
        let mut runner = JobRunner::new(queue.clone(), router, concurrency);
        tokio::spawn(async move {
            if let Err(e) = runner.run(Duration::milliseconds(1)).await {
                eprintln!("Runner crashed: {}", e);
            }
        })
    };

    let mut results = rx
        .take(count.try_into().unwrap())
        .collect::<Vec<JobResult>>()
        .await;

    let seen_ids = { results.iter().map(|r| r.jid).collect::<HashSet<Xid>>() };

    if seen_ids.len() < results.len() {
        eprintln!("Got some duplicates yo");
    }

    let total_duration = started.elapsed();
    results.sort_by_key(|r| r.duration_millis);
    let throughput = count as f64 / total_duration.as_secs_f64();
    let (min, max, median, pct) = (
        results[0].duration_millis,
        results[count - 1].duration_millis,
        results[count / 2].duration_millis,
        results[(count * 19) / 20].duration_millis,
    );

    println!("Processed: {} jobs", processed.load(Ordering::SeqCst));
    println!("min: {}ms", min);
    println!("max: {}ms", max);
    println!("median: {}ms", median);
    println!("95th percentile: {}ms", pct);
    println!("throughput: {}/s", throughput);
    queue.print_job_queue().await.unwrap();
}
