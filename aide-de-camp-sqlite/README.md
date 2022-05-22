# aide-de-camp-sqlite

A SQLite backed implementation of the job Queue.

NOTE: It is possible that a single job gets sent to two runners. This is due to SQLite lacking
row locking and `BEGIN EXCLUSIVE TRANSACTION` not working well (it was very slow) for this use
case. This is only an issue at high concurrency, in which case you probably don't want to use
SQLite in the first place. In other words, this isn't "Exactly Once" kind of queue.

## Schema

```sql
CREATE TABLE IF NOT EXISTS adc_queue (
 jid TEXT PRIMARY KEY,
 queue TEXT NOT NULL default 'default',
 job_type TEXT not null,
 payload blob not null,
 retries int not null default 0,
 scheduled_at INTEGER not null,
 started_at INTEGER,
 enqueued_at INTEGER not null default (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS adc_dead_queue (
 jid TEXT PRIMARY KEY,
 queue TEXT NOT NULL,
 job_type TEXT not null,
 payload blob not null,
 retries int not null,
 scheduled_at INTEGER not null,
 started_at INTEGER not null,
 enqueued_at INTEGER not null,
 died_at INTEGER not null default (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS adc_queue_jobs ON adc_queue (
    scheduled_at asc,
    started_at asc,
    queue,
    job_type
);
```

Schema also included into this crate as `SCHEMA_SQL` constant in this crate.

## Example

```rust

use aide_de_camp_sqlite::{SqliteQueue, SCHEMA_SQL};
use aide_de_camp::prelude::{Queue, JobHandler, JobRunner, RunnerRouter, Duration, Xid};
use async_trait::async_trait;
use sqlx::SqlitePool;

struct MyJob;

#[async_trait::async_trait]
impl JobHandler for MyJob {
    type Payload = Vec<u32>;
    type Error = anyhow::Error;

    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error> {
        // Do work here
        Ok(())
    }

    fn name() -> &'static str {
        "my_job"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let pool = SqlitePool::connect(":memory:").await?;
    // Setup schema, alternatively you can add schema to your migrations.
    sqlx::query(SCHEMA_SQL).execute(&pool).await?;
    let queue = SqliteQueue::with_pool(pool);

    // Add job the queue to run next
    let _jid = queue.schedule::<MyJob>(vec![1,2,3]).await?;

    // First create a job processor and router
    let router = {
        let mut r = RunnerRouter::default();
        r.add_job_handler(MyJob);
        r
    };
    // Setup runner to at most 10 jobs concurrently
    let mut runner = JobRunner::new(queue, router, 10);
    // Poll the queue every second, this will block unless something went really wrong.
    // runner.run(Duration::seconds(1)).await?; // Commented so it doesn't endlessly block this doctest.
    Ok(())
}
```